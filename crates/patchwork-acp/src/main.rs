//! Patchwork ACP proxy - interprets Patchwork code in the ACP message chain.
//!
//! This proxy sits between an editor (like Zed) and an agent (like Claude Code),
//! intercepting prompts that contain Patchwork code and executing them with
//! integrated LLM support via think blocks.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sacp::schema::{ContentBlock, PromptRequest, PromptResponse, StopReason};
use sacp::{JrHandlerChain, JrRequestCx};
use sacp_proxy::{AcpProxyExt, JrCxExt, McpServiceRegistry};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing_subscriber::EnvFilter;

use patchwork_eval::{ControlState, Interpreter};

/// Session state for tracking active evaluations.
struct Session {
    /// The interpreter for this session's active evaluation.
    interpreter: Option<Interpreter>,
}

impl Session {
    fn new() -> Self {
        Self { interpreter: None }
    }

    fn has_active_evaluation(&self) -> bool {
        self.interpreter.is_some()
    }

    fn store_interpreter(&mut self, interp: Interpreter) {
        self.interpreter = Some(interp);
    }

    #[allow(dead_code)]
    fn take_interpreter(&mut self) -> Option<Interpreter> {
        self.interpreter.take()
    }
}

/// The Patchwork proxy state.
struct PatchworkProxy {
    /// Sessions indexed by session ID (for now, just use a single default session).
    sessions: HashMap<String, Session>,
}

impl PatchworkProxy {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    fn get_or_create_session(&mut self, session_id: &str) -> &mut Session {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(Session::new)
    }
}

/// Check if a message appears to be Patchwork code.
///
/// Patchwork code is identified by starting with `{` (after trimming whitespace).
fn is_patchwork_code(text: &str) -> bool {
    text.trim_start().starts_with('{')
}

/// Extract the text content from a prompt request.
fn extract_prompt_text(request: &PromptRequest) -> Option<String> {
    // The prompt request contains content blocks; look for Text blocks
    request.prompt.iter().find_map(|block| {
        if let ContentBlock::Text(text_content) = block {
            Some(text_content.text.clone())
        } else {
            None
        }
    })
}

/// Handle a prompt request, checking for Patchwork code.
async fn handle_prompt(
    proxy: Arc<Mutex<PatchworkProxy>>,
    request: PromptRequest,
    cx: JrRequestCx<PromptResponse>,
) -> Result<(), sacp::Error> {
    let session_id = request.session_id.to_string();

    // Extract the prompt text
    let Some(text) = extract_prompt_text(&request) else {
        // No text content, forward unchanged
        tracing::debug!("No text content in prompt, forwarding");
        cx.connection_cx()
            .send_request_to_successor(request)
            .forward_to_request_cx(cx)?;
        return Ok(());
    };

    // Check if it's Patchwork code
    if !is_patchwork_code(&text) {
        // Not code, forward unchanged
        tracing::debug!("Not Patchwork code, forwarding");
        cx.connection_cx()
            .send_request_to_successor(request)
            .forward_to_request_cx(cx)?;
        return Ok(());
    }

    tracing::info!("Detected Patchwork code, executing...");

    // Check for active evaluation
    {
        let mut proxy = proxy.lock().unwrap();
        let session = proxy.get_or_create_session(&session_id);

        if session.has_active_evaluation() {
            // Already evaluating, return error
            cx.respond_with_error(
                sacp::Error::invalid_request()
                    .with_data("Patchwork evaluation already in progress"),
            )?;
            return Ok(());
        }
    }

    // Create interpreter and evaluate
    let mut interp = Interpreter::new();
    match interp.eval(&text) {
        Ok(state) => {
            match state {
                ControlState::Return(value) => {
                    tracing::info!("Patchwork code completed: {:?}", value);
                    // For now, just respond with a success message
                    // In Phase 3, we'll construct a proper response
                    let response = create_text_response(format!(
                        "Patchwork execution completed: {}",
                        value
                    ));
                    cx.respond(response)?;
                }
                ControlState::Throw(value) => {
                    tracing::error!("Patchwork code threw: {:?}", value);
                    cx.respond_with_error(
                        sacp::Error::internal_error()
                            .with_data(format!("Patchwork error: {}", value)),
                    )?;
                }
                ControlState::Yield { prompt, .. } => {
                    tracing::info!("Patchwork code yielded for LLM: {}", prompt);
                    // Store interpreter for resume
                    {
                        let mut proxy = proxy.lock().unwrap();
                        let session = proxy.get_or_create_session(&session_id);
                        session.store_interpreter(interp);
                    }
                    // TODO: In Phase 3, forward the prompt to successor
                    cx.respond_with_error(
                        sacp::Error::internal_error()
                            .with_data("Think blocks not yet implemented"),
                    )?;
                }
                ControlState::Eval => {
                    // Shouldn't happen after eval() returns
                    cx.respond_with_error(
                        sacp::Error::internal_error()
                            .with_data("Unexpected Eval state after execution"),
                    )?;
                }
            }
        }
        Err(e) => {
            tracing::error!("Patchwork parse/eval error: {}", e);
            cx.respond_with_error(
                sacp::Error::invalid_params().with_data(format!("Patchwork error: {}", e)),
            )?;
        }
    }

    Ok(())
}

/// Create a simple text response.
fn create_text_response(_text: String) -> PromptResponse {
    // TODO: In a full implementation, we'd need to send progress notifications
    // with the text content, since PromptResponse only contains stop_reason.
    // For now, we just log and return success.
    PromptResponse {
        stop_reason: StopReason::EndTurn,
        meta: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Patchwork ACP proxy");

    // Create shared proxy state
    let proxy = Arc::new(Mutex::new(PatchworkProxy::new()));

    // Build the handler chain
    let proxy_clone = Arc::clone(&proxy);
    JrHandlerChain::new()
        .name("patchwork-acp")
        .on_receive_request(move |request: PromptRequest, cx: JrRequestCx<PromptResponse>| {
            let proxy = Arc::clone(&proxy_clone);
            async move { handle_prompt(proxy, request, cx).await }
        })
        .provide_mcp(McpServiceRegistry::default())
        .proxy()
        .connect_to(sacp::ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))?
        .serve()
        .await?;

    Ok(())
}
