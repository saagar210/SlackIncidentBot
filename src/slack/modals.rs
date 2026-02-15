use serde_json::{json, Value};

pub fn declare_incident_modal(services: &[String]) -> Value {
    let service_options: Vec<Value> = services
        .iter()
        .map(|s| {
            json!({
                "text": {
                    "type": "plain_text",
                    "text": s,
                },
                "value": s,
            })
        })
        .collect();

    json!({
        "type": "modal",
        "callback_id": "declare_incident_modal",
        "title": {
            "type": "plain_text",
            "text": "Declare Incident",
        },
        "submit": {
            "type": "plain_text",
            "text": "Declare",
        },
        "close": {
            "type": "plain_text",
            "text": "Cancel",
        },
        "blocks": [
            {
                "type": "input",
                "block_id": "title_block",
                "label": {
                    "type": "plain_text",
                    "text": "Incident Title",
                },
                "element": {
                    "type": "plain_text_input",
                    "action_id": "title_input",
                    "placeholder": {
                        "type": "plain_text",
                        "text": "e.g., Okta SSO outage",
                    },
                    "max_length": 100,
                },
            },
            {
                "type": "input",
                "block_id": "severity_block",
                "label": {
                    "type": "plain_text",
                    "text": "Severity",
                },
                "element": {
                    "type": "static_select",
                    "action_id": "severity_select",
                    "initial_option": {
                        "text": {
                            "type": "plain_text",
                            "text": "P2 (High)",
                        },
                        "value": "P2",
                    },
                    "options": [
                        {
                            "text": {
                                "type": "plain_text",
                                "text": "P1 (Critical)",
                            },
                            "value": "P1",
                        },
                        {
                            "text": {
                                "type": "plain_text",
                                "text": "P2 (High)",
                            },
                            "value": "P2",
                        },
                        {
                            "text": {
                                "type": "plain_text",
                                "text": "P3 (Medium)",
                            },
                            "value": "P3",
                        },
                        {
                            "text": {
                                "type": "plain_text",
                                "text": "P4 (Low)",
                            },
                            "value": "P4",
                        },
                    ],
                },
            },
            {
                "type": "input",
                "block_id": "service_block",
                "label": {
                    "type": "plain_text",
                    "text": "Affected Service",
                },
                "element": {
                    "type": "static_select",
                    "action_id": "service_select",
                    "options": service_options,
                },
            },
            {
                "type": "input",
                "block_id": "commander_block",
                "label": {
                    "type": "plain_text",
                    "text": "Incident Commander",
                },
                "element": {
                    "type": "users_select",
                    "action_id": "commander_select",
                },
                "optional": true,
            },
        ],
    })
}
