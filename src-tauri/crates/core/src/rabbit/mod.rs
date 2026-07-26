//! RabbitMQ 客户端：Management HTTP API + AMQP publisher + 手动消费者

pub mod consumer_manager;
pub mod management;
pub mod publisher;

pub use consumer_manager::ConsumerManager;
pub use management::ManagementClient;
pub use publisher::AmqpPublisher;
