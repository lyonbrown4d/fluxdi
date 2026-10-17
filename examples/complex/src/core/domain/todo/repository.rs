use crate::core::domain::todo::Todo;

#[async_trait::async_trait]
pub trait TodoRepository: Send + Sync {
    async fn get_all(&self) -> Result<Vec<Todo>, String>;

    async fn get_by_id(&self, id: u32) -> Result<Option<Todo>, String>;

    async fn create(
        &self,
        user_id: u32,
        title: String,
        description: String,
    ) -> Result<Todo, String>;

    async fn update_status(&self, id: u32, completed: bool) -> Result<Option<Todo>, String>;

    async fn delete(&self, id: u32) -> Result<bool, String>;
}
