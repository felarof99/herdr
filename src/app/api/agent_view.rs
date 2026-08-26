use crate::api::schema::{AgentViewClearParams, AgentViewSetParams, ResponseResult};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_agent_view_set(
        &mut self,
        id: String,
        mut params: AgentViewSetParams,
    ) -> String {
        if let Err(message) = crate::app::agent_view::validate_agent_view(&mut params) {
            return encode_error(id, "invalid_agent_view", message);
        }
        if let Some(plugin_id) = params.source.strip_prefix("plugin:") {
            let Some(plugin_id) = super::plugins::normalize_plugin_id(plugin_id) else {
                return encode_error(
                    id,
                    "invalid_agent_view",
                    "plugin-owned agent view source has an invalid plugin id",
                );
            };
            let Some(plugin) = self.state.installed_plugins.get(&plugin_id) else {
                return encode_error(id, "plugin_not_found", "plugin not found");
            };
            if !super::plugins::plugin_manifest_available(plugin) {
                return encode_error(
                    id,
                    "plugin_manifest_unavailable",
                    format!("plugin {plugin_id} manifest is unavailable"),
                );
            }
            if !plugin.enabled {
                return encode_error(id, "plugin_disabled", "plugin is disabled");
            }
        }
        let source = params.source.clone();
        let label = params.label.clone();
        self.replace_agent_view_override(Some(params));
        encode_success(
            id,
            ResponseResult::AgentView {
                active: true,
                source: Some(source),
                label,
            },
        )
    }

    pub(super) fn handle_agent_view_clear(
        &mut self,
        id: String,
        params: AgentViewClearParams,
    ) -> String {
        let source = match params.source {
            Some(source) => match crate::app::agent_view::validate_agent_view_source(&source) {
                Ok(source) => Some(source),
                Err(message) => return encode_error(id, "invalid_agent_view", message),
            },
            None => None,
        };
        if source.as_deref().is_none_or(|source| {
            self.state
                .agent_view_override
                .as_ref()
                .is_some_and(|active| active.source == source)
        }) {
            self.replace_agent_view_override(None);
        }
        let active = self.state.agent_view_override.as_ref();
        encode_success(
            id,
            ResponseResult::AgentView {
                active: active.is_some(),
                source: active.map(|view| view.source.clone()),
                label: active.and_then(|view| view.label.clone()),
            },
        )
    }

    pub(crate) fn clear_agent_view_for_source(&mut self, source: &str) -> bool {
        if self
            .state
            .agent_view_override
            .as_ref()
            .is_some_and(|active| active.source == source)
        {
            self.replace_agent_view_override(None);
            true
        } else {
            false
        }
    }

    fn replace_agent_view_override(&mut self, view: Option<AgentViewSetParams>) {
        self.state.agent_view_override = view;
        self.state.plugin_agent_view_selection = None;
        self.state.agent_panel_scroll = 0;
        self.state.mobile_switcher_scroll = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{
        AgentViewBuiltinField, AgentViewField, AgentViewFilter, AgentViewValue, InstalledPluginInfo,
    };

    fn test_app() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    fn working_view(source: &str) -> AgentViewSetParams {
        AgentViewSetParams {
            source: source.to_string(),
            label: Some("working".to_string()),
            filter: Some(AgentViewFilter::Eq {
                field: AgentViewField::Builtin(AgentViewBuiltinField::Status),
                value: AgentViewValue::String("working".to_string()),
            }),
            sort: Vec::new(),
        }
    }

    #[test]
    fn set_and_source_guarded_clear_replace_transient_view() {
        let mut app = test_app();

        let set = app.handle_agent_view_set("set".to_string(), working_view("example.views"));
        let set: crate::api::schema::SuccessResponse = serde_json::from_str(&set).unwrap();
        assert_eq!(
            set.result,
            ResponseResult::AgentView {
                active: true,
                source: Some("example.views".to_string()),
                label: Some("working".to_string()),
            }
        );
        assert_eq!(
            app.state
                .agent_view_override
                .as_ref()
                .map(|view| view.source.as_str()),
            Some("example.views")
        );

        app.handle_agent_view_clear(
            "wrong-source".to_string(),
            AgentViewClearParams {
                source: Some("other.views".to_string()),
            },
        );
        assert!(app.state.agent_view_override.is_some());

        app.handle_agent_view_clear(
            "right-source".to_string(),
            AgentViewClearParams {
                source: Some("example.views".to_string()),
            },
        );
        assert!(app.state.agent_view_override.is_none());
    }

    #[test]
    fn invalid_view_does_not_replace_active_view() {
        let mut app = test_app();
        app.handle_agent_view_set("set".to_string(), working_view("example.views"));

        let mut invalid = working_view("example.other");
        invalid.filter = Some(AgentViewFilter::Any {
            filters: Vec::new(),
        });
        let response = app.handle_agent_view_set("invalid".to_string(), invalid);
        let response: crate::api::schema::ErrorResponse = serde_json::from_str(&response).unwrap();

        assert_eq!(response.error.code, "invalid_agent_view");
        assert_eq!(
            app.state
                .agent_view_override
                .as_ref()
                .map(|view| view.source.as_str()),
            Some("example.views")
        );
    }

    #[test]
    fn unavailable_plugin_cannot_set_agent_view() {
        let mut app = test_app();
        app.state.installed_plugins.insert(
            "example.views".into(),
            InstalledPluginInfo {
                plugin_id: "example.views".into(),
                name: "Example Views".into(),
                version: "0.1.0".into(),
                min_herdr_version: crate::build_info::BASE_VERSION.into(),
                description: None,
                manifest_path: "/missing/herdr-plugin.toml".into(),
                plugin_root: "/missing".into(),
                enabled: true,
                platforms: None,
                build: Vec::new(),
                startup: Vec::new(),
                actions: Vec::new(),
                events: Vec::new(),
                panes: Vec::new(),
                agent_views: Vec::new(),
                link_handlers: Vec::new(),
                source: Default::default(),
                warnings: vec![format!(
                    "{}missing manifest",
                    crate::persist::plugin_registry::MANIFEST_UNAVAILABLE_WARNING_PREFIX
                )],
            },
        );

        let response =
            app.handle_agent_view_set("unavailable".into(), working_view("plugin:example.views"));
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response["error"]["code"], "plugin_manifest_unavailable");
        assert!(app.state.agent_view_override.is_none());
    }
}
