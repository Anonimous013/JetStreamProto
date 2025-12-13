use crate::crd::JetStreamServer;
use kube::{
    api::{Api, Patch, PatchParams, ResourceExt},
    client::Client,
    runtime::controller::{Action, Controller},
    Error,
};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use serde_json::json;
use std::sync::Arc;
use tokio::time::Duration;
use futures::StreamExt;

pub struct Context {
    pub client: Client,
}

async fn reconcile(server: Arc<JetStreamServer>, ctx: Arc<Context>) -> Result<Action, Error> {
    let client = ctx.client.clone();
    let name = server.name_any();
    let namespace = server.namespace().unwrap_or("default".into());
    let spec = &server.spec;

    // Create/Update Deployment
    let deployment_json = json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": {
                "app": name
            }
        },
        "spec": {
            "replicas": spec.replicas,
            "selector": {
                "matchLabels": {
                    "app": name
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": name
                    }
                },
                "spec": {
                    "containers": [{
                        "name": "server",
                        "image": spec.image,
                        "ports": [{
                            "containerPort": spec.port
                        }]
                    }]
                }
            }
        }
    });
    let deployment: Deployment = serde_json::from_value(deployment_json).unwrap();

    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    deployments.patch(
        &name,
        &PatchParams::apply("jsp-controller"),
        &Patch::Apply(&deployment),
    ).await?;

    // Create/Update Service
    let service: k8s_openapi::api::core::v1::Service = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": {
                "app": name
            }
        },
        "spec": {
            "selector": {
                "app": name
            },
            "ports": [{
                "protocol": "UDP",
                "port": spec.port,
                "targetPort": spec.port
            }],
            "type": "LoadBalancer"
        }
    })).unwrap();

    let services: Api<Service> = Api::namespaced(client.clone(), &namespace);
    services.patch(
        &name,
        &PatchParams::apply("jsp-controller"),
        &Patch::Apply(&service),
    ).await?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(_server: Arc<JetStreamServer>, _error: &Error, _ctx: Arc<Context>) -> Action {
    Action::requeue(Duration::from_secs(30))
}

pub async fn run(client: Client) {
    let servers = Api::<JetStreamServer>::all(client.clone());
    let context = Arc::new(Context { client });

    Controller::new(servers, kube::runtime::watcher::Config::default())
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => tracing::info!("reconciled {:?}", o),
                Err(e) => tracing::error!("reconcile failed: {:?}", e),
            }
        })
        .await;
}
