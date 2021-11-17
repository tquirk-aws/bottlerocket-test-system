/*!
 *
This program takes its input (the "spec") and writes it to its output (the "created resource"). The
purpose of this program is to test the resources that depend on other resources for their inputs,
and tests that depend on resources for their inputs.

!*/

use model::Configuration;
use resource_agent::clients::InfoClient;
use resource_agent::provider::{
    Create, Destroy, IntoProviderError, ProviderResult, Resources, Spec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Memo {
    info: Option<DuplicationRequest>,
}

impl Configuration for Memo {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct DuplicationRequest {
    /// The info that will be copied to `DuplicatedData`.
    pub info: Value,
}

impl Configuration for DuplicationRequest {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct DuplicatedData {
    /// The info we have duplicated.
    info: Value,
}

impl Configuration for DuplicatedData {}

pub struct DuplicationCreator {}

#[async_trait::async_trait]
impl Create for DuplicationCreator {
    type Info = Memo;
    type Request = DuplicationRequest;
    type Resource = DuplicatedData;

    async fn create<I>(
        &self,
        request: Spec<Self::Request>,
        client: &I,
    ) -> ProviderResult<Self::Resource>
    where
        I: InfoClient,
    {
        let mut memo: Memo = client
            .get_info()
            .await
            .context(Resources::Clear, "Unable to get info from client")?;
        memo.info = Some(request.configuration.clone());
        client.send_info(memo.clone()).await.context(
            Resources::Remaining,
            "Error sending cluster created message",
        )?;
        Ok(DuplicatedData {
            info: request.configuration.info.clone(),
        })
    }
}

pub struct DuplicationDestroyer {}
#[async_trait::async_trait]
impl Destroy for DuplicationDestroyer {
    type Request = DuplicationRequest;
    type Info = Memo;
    type Resource = DuplicatedData;

    async fn destroy<I>(
        &self,
        _request: Option<Spec<Self::Request>>,
        _resource: Option<Self::Resource>,
        _client: &I,
    ) -> ProviderResult<()>
    where
        I: InfoClient,
    {
        // Nothing to destroy.
        Ok(())
    }
}