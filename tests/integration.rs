#![allow(dead_code)]

mod cli {
    pub mod commands {
        pub mod init {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/cli/commands/init.rs"
            ));
        }

        pub mod pack {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/cli/commands/pack.rs"
            ));
        }

        use clap::{Args, Parser, Subcommand};
        use clap_verbosity_flag::{Verbosity, WarnLevel};

        #[derive(Parser, Debug)]
        #[command(name = "rbxex", version, about, propagate_version = true)]
        pub struct Cli {
            #[command(flatten)]
            pub global: GlobalOptions,

            #[command(subcommand)]
            pub command: Option<Commands>,
        }

        #[derive(Subcommand, Debug)]
        pub enum Commands {
            Pack(pack::PackArgs),
            Init(init::InitArgs),
        }

        #[derive(Args, Debug, Clone, Default)]
        pub struct GlobalOptions {
            #[command(flatten)]
            pub verbosity: Verbosity<WarnLevel>,
        }

        pub mod prelude {
            pub use anyhow::Result;
            pub use clap::{Parser, ValueEnum};
            pub use std::path::PathBuf;
        }
    }

    pub mod ops {
        pub mod init {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/ops/init.rs"));
        }

        pub mod pack {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli/ops/pack.rs"));
        }
    }

    pub mod utils {
        pub mod command {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/cli/utils/command.rs"
            ));
        }

        pub mod rojo {
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/cli/utils/rojo.rs"
            ));
        }
    }
}

#[path = "integration/mod.rs"]
mod integration;
