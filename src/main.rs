

use futures::{StreamExt, TryStreamExt};
use kube::{
    api::{Api, ApiResource, DynamicObject, GroupVersionKind, ListParams, PostParams},
    discovery::{self},
    runtime::{watcher, WatchStreamExt},
    Client, 
};

//use tracing::*;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::try_default().await?;

    // Take dynamic resource identifiers:
    let group = env::var("GROUP").unwrap_or_else(|_| "elemental.cattle.io".into());
    let version = env::var("VERSION").unwrap_or_else(|_| "v1beta1".into());
    let kind = env::var("KIND").unwrap_or_else(|_| "MachineInventory".into());

    // Turn them into a GVK
    let gvk = GroupVersionKind::gvk(&group, &version, &kind);
    // Use API discovery to identify more information about the type (like its plural)
    let (ar, caps) = discovery::pinned_kind(&client, &gvk).await?;

    // Use the full resource info to create an Api with the ApiResource as its DynamicType
    let api = Api::<DynamicObject>::all_with(client.clone(), &ar);

    // Fully compatible with kube-runtime
    let mut items = watcher(api, ListParams::default()).applied_objects().boxed();


    while let Some(p) = items.try_next().await? {
        if let Some(labels) = p.metadata.labels {
            if let Some(cluster_name) = labels.get("autoClusterName") {
                build_all_if_not_exists(client.clone(), cluster_name.clone()).await;
            }
        }
    }
    Ok(())
}

async fn build_all_if_not_exists(client: Client, cluster_name: String) -> anyhow::Result<()> {

    //Check if cluster or selector already exist
    let exists_res = check_if_cluster_exists(client.clone(), cluster_name.clone()).await?;

    // Only run if we know it doesn't already exist
    if !exists_res {
        build_selector(client.clone(), cluster_name.clone()).await?;
        build_cluster(client.clone(), cluster_name.clone()).await?;
    }


    Ok(())
}

async fn check_if_cluster_exists(client: Client, cluster_name: String) -> anyhow::Result<bool> {
    let gvk = GroupVersionKind::gvk("provisioning.cattle.io", "v1", "Cluster");
    let api_resource = ApiResource::from_gvk(&gvk);
    let dynapi: Api<DynamicObject> = Api::namespaced_with(client.clone(), "fleet_default", &api_resource);

    let selector = String::from("metadata.name=") + cluster_name.as_str();
    let lp = &ListParams::default().fields(selector.as_str());

    let clusters = dynapi.list(lp).await?;

    if let Some(count) = clusters.metadata.remaining_item_count {
        return Ok(count>0)
    }


    let gvk = GroupVersionKind::gvk("provisioning.cattle.io", "v1", "Cluster");
    let api_resource = ApiResource::from_gvk(&gvk);
    let dynapi: Api<DynamicObject> = Api::namespaced_with(client.clone(), "fleet_default", &api_resource);

    let selector = String::from("metadata.name=") + cluster_name.as_str();
    let lp = &ListParams::default().fields(selector.as_str());

    let clusters = dynapi.list(lp).await?;

    if let Some(count) = clusters.metadata.remaining_item_count {
        return Ok(count>0)
    }

    Ok(false)
}

async fn build_cluster(client: Client, cluster_name: String) -> anyhow::Result<()> {
    let gvk = GroupVersionKind::gvk("elemental.cattle.io", "v1beta1", "MachineInventorySelectorTemplate");
    let api_resource = ApiResource::from_gvk(&gvk);
    let dynapi: Api<DynamicObject> = Api::namespaced_with(client.clone(), "fleet_default", &api_resource);

    let data = DynamicObject::new(&cluster_name, &api_resource).data(serde_json::json!({
        "spec": {
            "rkeConfig": {
                "machinePools": [{
                    "controlPlaneRole": true,
                    "name": "pool1",
                    "quantity": 5,
                    "workerRole": true,
                    "machineConfigRef":{
                        "apiVersion": "elemental.cattle.io/v1beta1",
                        "kind": "MachineInventorySelectorTemplate",
                        "name": cluster_name
                    }
                }]
            },
            "kubernetesVersion": "v1.24.8+k3s1"
                
        }
    }));

    let _res = dynapi.create(&PostParams::default(), &data).await?;

    Ok(())
}

async fn build_selector(client: Client, cluster_name: String) -> anyhow::Result<()> {
    let gvk = GroupVersionKind::gvk("elemental.cattle.io", "v1beta1", "MachineInventorySelectorTemplate");
    let api_resource = ApiResource::from_gvk(&gvk);
    let dynapi: Api<DynamicObject> = Api::namespaced_with(client.clone(), "fleet_default", &api_resource);

    let data = DynamicObject::new(&cluster_name, &api_resource).data(serde_json::json!({
        "spec": {
            "template": {
                "spec": {
                    "selector": {
                        "matchLabels":{
                            "autoClusterName": cluster_name
                        }
                    }
                }
            }
                
        }
    }));

    let _res = dynapi.create(&PostParams::default(), &data).await?;
    Ok(())
}