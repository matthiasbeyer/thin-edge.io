#[async_trait::async_trait]
pub trait Task {
    async fn run(self) -> miette::Result<()>;
}
