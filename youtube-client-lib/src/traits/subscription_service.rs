use crate::models::Subscription;
use crate::Result;

pub trait SubscriptionService: Send + Sync {
    async fn list_subscriptions(&self, max_results: u32) -> Result<Vec<Subscription>>;
    async fn subscribe_to_channel(&self, channel_id: &str) -> Result<()>;
    async fn unsubscribe_from_channel(&self, subscription_id: &str) -> Result<()>;
}
