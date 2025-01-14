use solana_program::declare_id;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

pub mod error;
pub mod instruction;
pub mod processor;
pub mod macros;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

declare_id!("5TAiuAh3YGDbwjEruC1ZpXTJWdNDS7Ur7VeqNNiHMmGV");

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Jito Stake Deposit Interceptor Program",
    project_url: "https://jito.network/",
    contacts: "email:support@jito.network",
    policy: "https://github.com/jito-foundation/stake-deposit-interceptor",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/jito-foundation/stake-deposit-interceptor",
    source_revision: std::env!("GIT_SHA"),
    source_release: std::env!("GIT_REF_NAME")
}
