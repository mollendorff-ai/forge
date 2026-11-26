//! Every MCP tool over REST, and the `OpenAPI` document of the API (ADR-081).
//!
//! An MCP tool that has no endpoint of its own is served at
//! `POST /api/v1/<name>` (`forge_goal_seek` at `/api/v1/goal-seek`): the body
//! is the tool's arguments, as `tools/call` takes them, and the answer is the
//! tool's JSON in `data`. The endpoint runs the tool itself
//! ([`ForgeMcpServer::call`]), so the two surfaces share the request type, the
//! method and the command's core function. The routes and the document are
//! made from the tool list, so a new tool gets both with nothing else to
//! write.

use std::sync::Arc;

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use rmcp::schemars::{self, JsonSchema};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::handlers::{self, ApiResponse};
use super::server::AppState;
use crate::mcp::ForgeMcpServer;

/// The endpoints written before every tool had one, which keep their own
/// requests and answers (ADR-081, decision 4)
pub const OWN: [&str; 5] = ["validate", "calculate", "audit", "export", "import"];

/// The path of a tool's endpoint: `forge_goal_seek` is `/api/v1/goal-seek`
#[must_use]
pub fn path_of(tool: &str) -> String {
    format!(
        "/api/v1/{}",
        tool.trim_start_matches("forge_").replace('_', "-")
    )
}

/// The tools served through [`ForgeMcpServer::call`]: all of them but those
/// whose path is one of [`OWN`]
#[must_use]
pub fn served(server: &ForgeMcpServer) -> Vec<rmcp::model::Tool> {
    let own: Vec<String> = OWN.iter().map(|name| format!("/api/v1/{name}")).collect();
    server
        .tools()
        .into_iter()
        .filter(|tool| !own.contains(&path_of(&tool.name)))
        .collect()
}

/// A route for each served tool, each running it on `server`
pub fn routes(server: &ForgeMcpServer) -> Router<Arc<AppState>> {
    let shared = Arc::new(server.clone());
    served(server)
        .into_iter()
        .fold(Router::new(), |router, tool| {
            let server = Arc::clone(&shared);
            let name = tool.name.to_string();
            router.route(
                &path_of(&name),
                post(move |body: Bytes| answer(server, name, body)),
            )
        })
}

/// What a served tool answers: `{success: true, request_id, data}` with the
/// tool's JSON; a body that is not JSON is 400, and a tool that fails, or
/// arguments that are not its request, 422 with the tool's message
async fn answer(server: Arc<ForgeMcpServer>, name: String, body: Bytes) -> Response {
    // No body is no arguments, as for a tool that takes none
    let arguments = if body.is_empty() {
        Ok(json!({}))
    } else {
        serde_json::from_slice::<Value>(&body)
    };
    let Ok(arguments) = arguments else {
        return failed(
            StatusCode::BAD_REQUEST,
            "The body is not JSON: send the tool's arguments as an object",
        );
    };
    // A tool reads files and calculates: it runs off the server's threads
    let outcome = tokio::task::spawn_blocking(move || server.call(&name, arguments))
        .await
        .expect("a tool answers or fails, it does not panic");
    match outcome {
        Ok(text) => {
            let data: Value = serde_json::from_str(&text)
                .expect("every tool answers JSON (the parity test reads each one)");
            Json(ApiResponse::ok(data)).into_response()
        },
        Err(message) => failed(StatusCode::UNPROCESSABLE_ENTITY, &message),
    }
}

/// An answer that says why nothing was done
fn failed(status: StatusCode, error: &str) -> Response {
    (
        status,
        Json(json!({
            "success": false,
            "request_id": Uuid::new_v4().to_string(),
            "error": error,
        })),
    )
        .into_response()
}

/// The `OpenAPI` 3.1 document of the API.
///
/// It is made from the tools and the request types of the endpoints of their
/// own: each body's schema is the one MCP's `tools/list` gives the tool, or
/// schemars' for the request, so it cannot say something else than what the
/// server reads.
#[must_use]
pub fn openapi(server: &ForgeMcpServer) -> Value {
    let mut schemas = Map::new();
    let mut paths = Map::new();
    let own: [(&str, &str, Value); 5] = [
        (
            "validate",
            "Check each stored value against its formula, and each expectation; `valid` says whether all hold",
            schema::<handlers::ValidateRequest>(),
        ),
        (
            "calculate",
            "Calculate the model's formulas; the file is written unless `dry_run`, and inline content is answered calculated",
            schema::<handlers::CalculateRequest>(),
        ),
        (
            "audit",
            "A variable's dependency tree and its calculated value",
            schema::<handlers::AuditRequest>(),
        ),
        (
            "export",
            "Write the model as an Excel workbook at `excel_path`",
            schema::<handlers::ExportRequest>(),
        ),
        (
            "import",
            "Read an Excel workbook into a model at `yaml_path`",
            schema::<handlers::ImportRequest>(),
        ),
    ];
    for (name, description, body) in own {
        let operation = operation(name, description, body, &mut schemas, false);
        paths.insert(format!("/api/v1/{name}"), json!({ "post": operation }));
    }
    for tool in served(server) {
        let description = tool.description.as_deref().unwrap_or_default();
        let name = tool.name.trim_start_matches("forge_");
        let body = Value::Object((*tool.input_schema).clone());
        let operation = operation(name, description, body, &mut schemas, true);
        paths.insert(path_of(&tool.name), json!({ "post": operation }));
    }
    fixed(&mut paths);
    envelopes(&mut schemas);
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Forge API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Forge's REST API (forge serve): every MCP tool at /api/v1/<name>, its body the tool's arguments and its answer the tool's JSON in `data` (ADR-081). Requests name files on the machine running Forge, or send the model as `content`.",
        },
        "paths": paths,
        "components": {
            "schemas": schemas,
            "securitySchemes": {"token": {
                "type": "http",
                "scheme": "bearer",
                "description": "Required on /api/v1/* when the server has a token (FORGE_API_TOKEN, --token)",
            }},
        },
        // A server on this machine may have no token
        "security": [{}, {"token": []}],
    })
}

/// The endpoints that are not tools: the root, health, version and this
/// document
fn fixed(paths: &mut Map<String, Value>) {
    for (path, description) in [
        ("/", "The endpoints"),
        ("/health", "Whether the server is running"),
        ("/version", "The server's version"),
    ] {
        paths.insert(
            path.to_string(),
            json!({"get": {
                "operationId": (if path == "/" { "root" } else { path.trim_start_matches('/') }),
                "description": description,
                "security": [],
                "responses": {"200": answered("The answer, in `data`")},
            }}),
        );
    }
    paths.insert(
        "/api/v1/openapi.json".to_string(),
        json!({"get": {
            "operationId": "openapi",
            "description": "This document",
            "responses": {"200": {
                "description": "The OpenAPI document",
                "content": {"application/json": {"schema": {"type": "object"}}},
            }},
        }}),
    );
}

/// The answers' two shapes: `Answer` and `Failure`
fn envelopes(schemas: &mut Map<String, Value>) {
    schemas.insert(
        "Answer".to_string(),
        json!({
            "type": "object",
            "required": ["success", "request_id", "data"],
            "properties": {
                "success": {"const": true},
                "request_id": {"type": "string"},
                "data": {"description": "The answer: for a tool's endpoint, the JSON its MCP tool answers"},
            },
        }),
    );
    schemas.insert(
        "Failure".to_string(),
        json!({
            "type": "object",
            "required": ["success", "request_id", "error"],
            "properties": {
                "success": {"const": false},
                "request_id": {"type": "string"},
                "error": {"type": "string", "description": "Why nothing was done"},
            },
        }),
    );
}

/// The JSON Schema of a request type, as schemars writes it for MCP
fn schema<T: JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("a schema is JSON")
}

/// An operation whose body is `body`: its `$defs` go into the document's
/// schemas, and its references with them; a tool's endpoint fails with 400
/// and 422 (`fails`), an endpoint of its own answers its failures in `data`
fn operation(
    name: &str,
    description: &str,
    mut body: Value,
    schemas: &mut Map<String, Value>,
    fails: bool,
) -> Value {
    let object = body
        .as_object_mut()
        .expect("a request's schema is an object");
    object.remove("$schema");
    let mut defs = object.remove("$defs").unwrap_or_default();
    refer_to_components(&mut defs);
    for (key, def) in defs.as_object().into_iter().flatten() {
        schemas.insert(key.clone(), def.clone());
    }
    refer_to_components(&mut body);
    let mut responses = json!({"200": answered("The answer, in `data`")});
    if fails {
        responses["400"] = failure("The body is not JSON");
        responses["422"] =
            failure("The arguments are not the tool's, or the tool failed: `error` says why");
    }
    json!({
        "operationId": name.replace('-', "_"),
        "description": description,
        "requestBody": {
            "required": !fails,
            "content": {"application/json": {"schema": body}},
        },
        "responses": responses,
    })
}

/// `#/$defs/X` as `#/components/schemas/X`, everywhere in `value`
fn refer_to_components(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map.iter_mut() {
                if let (true, Value::String(target)) = (key == "$ref", &mut *inner) {
                    *target = target.replace("#/$defs/", "#/components/schemas/");
                }
                refer_to_components(inner);
            }
        },
        Value::Array(items) => items.iter_mut().for_each(refer_to_components),
        _ => {},
    }
}

/// A 200 answer
fn answered(description: &str) -> Value {
    json!({
        "description": description,
        "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Answer"}}},
    })
}

/// A failure's answer
fn failure(description: &str) -> Value {
    json!({
        "description": description,
        "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Failure"}}},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::server::{build_router, ApiConfig};
    use axum::body::Body;
    use axum::http::{header, Method, Request};
    use tower::ServiceExt;

    /// An example model's full path
    fn example(name: &str) -> String {
        format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))
    }

    /// What `router` answers to `method path` with `body`: its status and JSON
    async fn send(router: &Router, method: Method, path: &str, body: &str) -> (StatusCode, Value) {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header(header::HOST, "localhost")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("a request"),
            )
            .await
            .expect("the router answers");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("a body");
        (status, serde_json::from_slice(&bytes).unwrap_or_default())
    }

    /// Arguments for each served tool, for a run that succeeds
    fn arguments() -> Vec<(&'static str, Value)> {
        let starter = example("starter.yaml");
        vec![
            (
                "forge_edit",
                json!({"file_path": starter, "dry_run": true,
                "changes": [{"op": "set_value", "path": "plan.price", "value": 45}]}),
            ),
            (
                "forge_sensitivity",
                json!({"file_path": starter, "vary": "plan.price",
                "range": "30,50,10", "output": "plan.profit"}),
            ),
            (
                "forge_goal_seek",
                json!({"file_path": starter, "target": "plan.profit",
                "value": 0, "vary": "plan.price"}),
            ),
            (
                "forge_break_even",
                json!({"file_path": starter, "output": "plan.profit",
                "vary": "plan.price"}),
            ),
            (
                "forge_variance",
                json!({"budget_path": example("variance.yaml"),
                "actual_path": example("variance.yaml")}),
            ),
            (
                "forge_compare",
                json!({"file_path": example("scenarios.yaml"),
                "scenarios": ["base_case", "bull_case"]}),
            ),
            (
                "forge_scenarios",
                json!({"file_path": example("scenarios.yaml")}),
            ),
            (
                "forge_tornado",
                json!({"file_path": example("tornado.yaml")}),
            ),
            (
                "forge_game",
                json!({"file_path": example("games/stag-hunt.yaml")}),
            ),
            (
                "forge_simulate",
                json!({"file_path": example("monte-carlo.yaml"),
                "iterations": 200, "seed": 7}),
            ),
            (
                "forge_decision_tree",
                json!({"file_path": example("decision-tree.yaml")}),
            ),
            (
                "forge_real_options",
                json!({"file_path": example("real-options.yaml")}),
            ),
            (
                "forge_bootstrap",
                json!({"file_path": example("bootstrap.yaml"),
                "iterations": 200, "seed": 7}),
            ),
            (
                "forge_bayesian",
                json!({"file_path": example("bayesian.yaml")}),
            ),
            ("forge_schema", json!({})),
            ("forge_functions", json!({})),
            ("forge_templates", json!({})),
            ("forge_examples", json!({"name": "starter"})),
        ]
    }

    /// `value` without the time a run took, which differs from run to run
    fn untimed(mut value: Value) -> Value {
        if let Some(map) = value.as_object_mut() {
            map.remove("execution_time_ms");
            for inner in map.values_mut() {
                *inner = untimed(inner.take());
            }
        }
        value
    }

    /// Each served tool's endpoint answers what its MCP tool answers, its
    /// JSON in `data` (a failure's message is `a_failure_says_why...`)
    #[tokio::test]
    async fn every_tool_has_an_endpoint_that_answers_what_the_tool_answers() {
        let server = ForgeMcpServer::new();
        let router = build_router(&ApiConfig::default()).expect("the default starts");
        let tools: Vec<String> = served(&server).iter().map(|t| t.name.to_string()).collect();
        let named: Vec<String> = arguments().iter().map(|(n, _)| (*n).to_string()).collect();
        let (mut a, mut b) = (tools.clone(), named);
        a.sort();
        b.sort();
        assert_eq!(
            a, b,
            "every served tool, and only those, has arguments here"
        );
        // The five endpoints of their own are not served again
        assert_eq!(tools.len() + OWN.len(), server.tools().len());
        // Every one of these runs succeeds, and the endpoint answers the same
        for (name, args) in arguments() {
            let tool = server.call(name, args.clone());
            assert!(tool.is_ok(), "{name}: {tool:?}");
            let text = tool.unwrap_or_default();
            let (status, body) =
                send(&router, Method::POST, &path_of(name), &args.to_string()).await;
            assert_eq!(status, StatusCode::OK, "{name}: {body}");
            assert_eq!(body["success"], true);
            assert_eq!(
                untimed(body["data"].clone()),
                untimed(serde_json::from_str::<Value>(&text).unwrap_or_default()),
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn a_failure_says_why_with_its_status() {
        let router = build_router(&ApiConfig::default()).expect("the default starts");
        // A model that is not there: the tool's message
        let (status, body) = send(
            &router,
            Method::POST,
            "/api/v1/goal-seek",
            r#"{"file_path": "nowhere.yaml", "target": "a", "value": 0, "vary": "b"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["success"], false);
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Goal seek failed: "),
            "{body}"
        );
        assert!(body["request_id"].is_string());
        // Arguments that are not the tool's
        let (status, body) = send(
            &router,
            Method::POST,
            "/api/v1/goal-seek",
            r#"{"target": 1}"#,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .starts_with("Invalid parameters: "),
            "{body}"
        );
        // A body that is not JSON
        let (status, body) = send(&router, Method::POST, "/api/v1/schema", "version: v5").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            body["error"],
            "The body is not JSON: send the tool's arguments as an object"
        );
        // No body: no arguments
        let (status, body) = send(&router, Method::POST, "/api/v1/functions", "").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["data"]["categories"].is_array(), "{body}");
    }

    #[tokio::test]
    async fn the_token_and_the_root_hold_for_every_tool() {
        let folder = tempfile::tempdir().expect("a folder");
        let config = ApiConfig {
            token: Some("secret".to_string()),
            root: Some(folder.path().to_path_buf()),
            ..ApiConfig::default()
        };
        let router = build_router(&config).expect("a token and a root start");
        let body = json!({"file_path": example("starter.yaml"), "target": "plan.profit",
            "value": 0, "vary": "plan.price"})
        .to_string();
        let (status, _) = send(&router, Method::POST, "/api/v1/goal-seek", &body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/goal-seek")
                    .header(header::HOST, "localhost")
                    .header(header::AUTHORIZATION, "Bearer secret")
                    .body(Body::from(body))
                    .expect("a request"),
            )
            .await
            .expect("the router answers");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("a body");
        let body: Value = serde_json::from_slice(&bytes).unwrap_or_default();
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .contains("outside"),
            "{body}"
        );
    }

    /// Each reference of `value` names a schema the document has
    fn references_resolve(value: &Value, document: &Value, count: &mut usize) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(target)) = map.get("$ref") {
                    *count += 1;
                    assert!(
                        document.pointer(target.trim_start_matches('#')).is_some(),
                        "{target}"
                    );
                }
                map.values()
                    .for_each(|v| references_resolve(v, document, count));
            },
            Value::Array(items) => items
                .iter()
                .for_each(|v| references_resolve(v, document, count)),
            _ => {},
        }
    }

    #[tokio::test]
    async fn the_document_describes_every_endpoint_from_its_schema() {
        let server = ForgeMcpServer::new();
        let document = openapi(&server);
        assert_eq!(document["openapi"], "3.1.0");
        let paths = document["paths"].as_object().expect("paths");
        // Every tool, the root, health, version and the document itself
        assert_eq!(paths.len(), server.tools().len() + 4);
        let mut ids: Vec<&str> = paths
            .values()
            .flat_map(|ops| ops.as_object().expect("operations").values())
            .map(|op| op["operationId"].as_str().expect("an id"))
            .collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "operation ids are unique");
        // Each tool's body is the schema MCP lists for it, its $defs moved
        let edit = &paths["/api/v1/edit"]["post"];
        assert_eq!(edit["operationId"], "edit");
        let body = &edit["requestBody"]["content"]["application/json"]["schema"];
        assert_eq!(
            body["properties"]["changes"]["items"]["$ref"],
            "#/components/schemas/Change"
        );
        assert!(body.get("$defs").is_none() && body.get("$schema").is_none());
        let tool = server
            .tools()
            .into_iter()
            .find(|t| t.name == "forge_goal_seek")
            .expect("goal seek");
        let goal = &paths["/api/v1/goal-seek"]["post"];
        assert_eq!(
            goal["description"],
            tool.description.as_deref().unwrap_or_default()
        );
        let mut listed = Value::Object((*tool.input_schema).clone());
        listed.as_object_mut().expect("an object").remove("$schema");
        assert_eq!(
            goal["requestBody"]["content"]["application/json"]["schema"],
            listed
        );
        assert_eq!(
            goal["responses"]["422"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/Failure"
        );
        // An endpoint of its own: its request type's schema, its body required
        let audit = &paths["/api/v1/audit"]["post"];
        assert_eq!(audit["requestBody"]["required"], true);
        let fields = &audit["requestBody"]["content"]["application/json"]["schema"]["properties"];
        assert!(
            fields["variable"].is_object() && fields["file_path"].is_object(),
            "{fields}"
        );
        assert!(audit["responses"].get("422").is_none());
        // Every reference resolves, and every schema is JSON Schema 2020-12
        let mut refs = 0;
        references_resolve(&document, &document, &mut refs);
        assert!(refs > 20, "{refs}");
        for schema in document["components"]["schemas"]
            .as_object()
            .expect("schemas")
            .values()
        {
            assert!(jsonschema::draft202012::meta::is_valid(schema), "{schema}");
        }
        for op in paths
            .values()
            .flat_map(|ops| ops.as_object().expect("operations").values())
        {
            let body = &op["requestBody"]["content"]["application/json"]["schema"];
            assert!(
                body.is_null() || jsonschema::draft202012::meta::is_valid(body),
                "{body}"
            );
        }
        // The server answers it as it is
        let router = build_router(&ApiConfig::default()).expect("the default starts");
        let (status, answer) = send(&router, Method::GET, "/api/v1/openapi.json", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(answer, document);
    }

    #[tokio::test]
    async fn the_root_and_the_version_list_every_endpoint() {
        let router = build_router(&ApiConfig::default()).expect("the default starts");
        let (_, root) = send(&router, Method::GET, "/", "").await;
        let endpoints = root["data"]["endpoints"].as_array().expect("endpoints");
        let goal = endpoints
            .iter()
            .find(|e| e["path"] == "/api/v1/goal-seek")
            .expect("goal seek");
        assert_eq!(goal["method"], "POST");
        assert!(
            !goal["description"]
                .as_str()
                .unwrap_or_default()
                .ends_with('.'),
            "{goal}"
        );
        assert!(endpoints
            .iter()
            .any(|e| e["path"] == "/api/v1/openapi.json"));
        let (_, version) = send(&router, Method::GET, "/version", "").await;
        let features = version["data"]["features"].as_array().expect("features");
        assert_eq!(features.len(), ForgeMcpServer::new().tools().len());
        assert!(features.iter().any(|f| f == "decision-tree"));
    }
}
