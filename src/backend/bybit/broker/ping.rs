use super::Broker;




impl Broker {
    pub async fn ping(&self) -> Result<(), reqwest::Error> {
        self.client.get(&format!("{}/v2/public/time", self.auth.url)).send().await?;
        return Ok(());
    }
}