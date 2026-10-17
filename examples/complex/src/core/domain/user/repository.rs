use crate::core::domain::user::User;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_all(&self) -> Result<Vec<User>, String>;

    async fn get_by_id(&self, id: u32) -> Result<Option<User>, String>;

    async fn create(&self, name: String, email: String) -> Result<User, String>;

    async fn delete(&self, id: u32) -> Result<bool, String>;
}
