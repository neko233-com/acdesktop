use super::CommandContext;
use crate::{AppError, session};

impl CommandContext {
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.options.description = description;
        self
    }

    pub fn description(&self) -> Option<&str> {
        self.options.description.as_deref()
    }

    pub fn with_control_indicator_owner(mut self, owner: Option<String>) -> Self {
        self.options.indicator_owner = owner;
        self
    }

    pub fn control_indicator_owner(&self) -> Option<&str> {
        self.options.indicator_owner.as_deref()
    }

    pub fn with_agent_id(mut self, agent_id: Option<String>) -> Result<Self, AppError> {
        if self.options.agent_id == agent_id {
            return Ok(self);
        }
        if let Some(id) = agent_id.as_deref() {
            session::validate_agent_id(id)?;
        }
        self.options.agent_id = agent_id;
        if let Some(session_id) = self.session_id() {
            self.options.cursor_overlay = session::cursor_overlay_for_session_agent(
                Some(session_id),
                self.options.agent_id.as_deref(),
            )?;
        }
        Ok(self)
    }

    pub fn agent_id(&self) -> Option<&str> {
        self.options.agent_id.as_deref()
    }

    pub fn multi_agent(&self) -> bool {
        self.options.cursor_overlay.is_multi_agent()
    }
}
