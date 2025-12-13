use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Specification for JetStreamServer
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "jetstream.io", version = "v1", kind = "JetStreamServer", namespaced)]
#[kube(status = "JetStreamServerStatus")]
pub struct JetStreamServerSpec {
    pub replicas: i32,
    pub image: String,
    pub port: i32,
    pub config_map: Option<String>,
}

/// Status of JetStreamServer
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct JetStreamServerStatus {
    pub ready_replicas: i32,
    pub conditions: Vec<Condition>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Condition {
    pub type_: String,
    pub status: String,
    pub last_transition_time: String,
    pub reason: String,
    pub message: String,
}
