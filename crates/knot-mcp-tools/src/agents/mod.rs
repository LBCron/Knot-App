mod create;
mod lifecycle;
mod listing;
pub(crate) mod registry;
mod working_dir;

pub use create::create_agent;
pub use lifecycle::{close_agent, set_status};
pub use listing::{list_agents, register_agent};
pub use registry::describe_agents;
pub use working_dir::set_working_directory;

#[cfg(test)]
mod tests;
