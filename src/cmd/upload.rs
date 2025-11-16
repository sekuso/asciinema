use anyhow::Result;
use crate::cli;

#[cfg(feature = "no-internet")]
use crate::no_internet;

#[cfg(not(feature = "no-internet"))]
use crate::{config::Config, asciicast, api};
#[cfg(not(feature = "no-internet"))]
use tokio::runtime::Runtime;

impl cli::Upload {
    pub fn run(self) -> Result<()> {
        #[cfg(feature = "no-internet")]
        {
            // Don't attempt any network activity when the no-internet feature is enabled.
            return Err(no_internet::disabled().into());
        }

        #[cfg(not(feature = "no-internet"))]
        {
            Runtime::new()?.block_on(self.do_run())
        }
    }

    #[cfg(not(feature = "no-internet"))]
    async fn do_run(self) -> Result<()> {
        let mut config = Config::new(self.server_url.clone())?;
        let _ = asciicast::open_from_path(&self.file)?;
        let response = api::create_recording(&self.file, &mut config).await?;
        println!("{}", response.message.unwrap_or(response.url));

        Ok(())
    }
}
