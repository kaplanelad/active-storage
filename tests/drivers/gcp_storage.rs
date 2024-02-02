use active_storage::drivers;

const CONTAINER_NAME: &str = "test-container";
#[derive(Clone)]
struct MockClient {
    inner: drivers::inmem::InMemoryDriver,
}