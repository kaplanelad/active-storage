use std::{collections::HashMap, path::PathBuf};

use active_storage::{
    drivers::{aws_s3::AwsS3, Driver},
    StoreConfig,
};
use aws_sdk_s3::Client;
use aws_types::region::Region;
use dockertest_server::{servers::cloud::LocalStackServerConfig, Test};

use super::flow;

#[test]
fn aws_s3() {
    let env: HashMap<_, _> = vec![("SERVICES".to_string(), "s3".to_string())]
        .into_iter()
        .collect();
    let config = LocalStackServerConfig::builder()
        .env(env)
        .port(4562)
        .version("3.1".into())
        .build()
        .unwrap();
    let mut test = Test::new();
    test.register(config);

    test.run(|_instance| async move {
        let client = Client::from_conf(
            aws_sdk_s3::Config::builder()
                .force_path_style(true)
                .endpoint_url("http://127.0.0.1:4562/")
                .region(Region::new("us-west-2"))
                .build(),
        );

        let bucket = "test-bucket";

        client
            .create_bucket()
            .bucket(bucket.to_string())
            .send()
            .await
            .unwrap();

        let aws_s3_driver = Box::new(AwsS3::with_client(client, bucket)) as Box<dyn Driver>;

        let store = StoreConfig::with_driver(aws_s3_driver);

        flow::test_driver(&store, PathBuf::from(bucket)).await;
    });
}
