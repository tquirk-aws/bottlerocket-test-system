use crate::job::error::{JobError, JobResult};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, LocalObjectReference, PodSpec, PodTemplateSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::Api;
use model::constants::{
    APP_COMPONENT, APP_CREATED_BY, APP_INSTANCE, APP_MANAGED_BY, APP_NAME, APP_PART_OF, CONTROLLER,
    NAMESPACE, RESOURCE_AGENT, RESOURCE_AGENT_SERVICE_ACCOUNT, TESTSYS, TEST_AGENT,
    TEST_AGENT_SERVICE_ACCOUNT,
};
use model::Agent;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(crate) enum JobType {
    TestAgent,
    ResourceAgent,
}

#[derive(Debug, Clone)]
pub(crate) struct JobBuilder<'a> {
    pub(crate) agent: &'a Agent,
    pub(crate) job_name: &'a str,
    pub(crate) job_type: JobType,
    pub(crate) component: &'a str,
    pub(crate) environment_variables: Vec<(&'a str, String)>,
}

impl JobBuilder<'_> {
    pub(crate) async fn deploy(self, client: kube::Client) -> JobResult<Job> {
        let job = self.build();
        let api: Api<Job> = Api::namespaced(client, NAMESPACE);
        Ok(api
            .create(&PostParams::default(), &job)
            .await
            .map_err(JobError::create)?)
    }

    fn build(self) -> Job {
        let vars = env_vars(self.environment_variables);
        let labels = create_labels(self.job_type, &self.agent.name, self.job_name);

        Job {
            metadata: ObjectMeta {
                name: Some(self.job_name.into()),
                namespace: Some(NAMESPACE.to_owned()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(JobSpec {
                backoff_limit: Some(0),
                template: PodTemplateSpec {
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: self.job_name.into(),
                            image: Some(self.agent.image.to_owned()),
                            env: if vars.is_empty() { None } else { Some(vars) },
                            ..Container::default()
                        }],
                        restart_policy: Some(String::from("Never")),
                        image_pull_secrets: self.agent.pull_secret.as_ref().map(|secret| {
                            vec![LocalObjectReference {
                                name: Some(secret.into()),
                            }]
                        }),
                        service_account: Some(match self.job_type {
                            JobType::TestAgent => TEST_AGENT_SERVICE_ACCOUNT.to_owned(),
                            JobType::ResourceAgent => RESOURCE_AGENT_SERVICE_ACCOUNT.to_owned(),
                        }),
                        ..PodSpec::default()
                    }),
                    metadata: Some(ObjectMeta {
                        labels: Some(labels),
                        ..ObjectMeta::default()
                    }),
                },
                ..JobSpec::default()
            }),
            ..Job::default()
        }
    }
}

/// Creates the labels that we will add to the test pod deployment.
fn create_labels<S1, S2>(job_type: JobType, agent: S1, instance: S2) -> BTreeMap<String, String>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    [
        (APP_NAME, instance.as_ref()),
        (APP_INSTANCE, agent.as_ref()),
        (
            APP_COMPONENT,
            match job_type {
                JobType::TestAgent => TEST_AGENT,
                JobType::ResourceAgent => RESOURCE_AGENT,
            },
        ),
        (APP_PART_OF, TESTSYS),
        (APP_MANAGED_BY, CONTROLLER),
        (APP_CREATED_BY, CONTROLLER),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
    .collect()
}

fn env_vars(raw_vars: Vec<(&str, String)>) -> Vec<EnvVar> {
    raw_vars
        .into_iter()
        .map(|(name, value)| EnvVar {
            name: name.to_owned(),
            value: Some(value),
            value_from: None,
        })
        .collect()
}