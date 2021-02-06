use nvim_rs::Value;

use crate::{source};
use crate::{NeovimHandler, SharedNvim};
use log::{error, info, trace};

use anyhow::{Context, Result};

use nvim_rs::{
    call_args,
    compat::tokio::Compat,
    rpc::{model::IntoVal, unpack::TryUnpack},
    Neovim,
};

use tokio::{io::Stdout, sync::oneshot, task};

pub async fn notify(
    handler: &NeovimHandler,
    name: String,
    mut args: Vec<Value>,
    neovim: Neovim<Compat<Stdout>>,
) -> Result<()> {
    trace!("Notification: {}, {:?}", name, args);

    let nvim = SharedNvim::new(neovim);
    let completor = handler.completor();

    match name.as_ref() {
        "complete" => {
            // if completor.lock().await.quicker_than(Duration::from_millis(100)) {
            //     return;
            // }

            let mut abort_handle = handler.abort_handle.lock().await; // there is a redraw issue
            if let Some(ref mut previous_complete) = *abort_handle {
                info!("Aborting previous complete");
                previous_complete.abort();
            }

            let (tx, rx) = oneshot::channel();

            let nvim1 = nvim.clone();
            let join_handle = task::spawn(async move {
                info!("agregating");
                let mut completor = completor.lock().await;
                assert!(tx.send(completor.agregate_sources(nvim1).await?).is_ok());
                Ok(())
            });

            abort_handle.replace(join_handle);

            task::spawn(async move {
                if let Ok(entries) = rx.await {
                    nvim.call_function(
                        "complete",
                        call_args!(nvim.call_function("col", call_args!(".")).await?, entries),
                    )
                    .await?;

                    // nvim.call(
                    //     "nvim_complete",
                    //     call_args!(
                    //         nvim.call_function("col", call_args!(".")).await.unwrap(),
                    //         entries,
                    //         value!([=>]),
                    //     ),
                    // )
                    // .await
                    // .unwrap();
                }
                Ok::<_, anyhow::Error>(())
            });
        }

        "v_char" => {
            task::spawn(async move {
                let mut completor_handle = completor.lock().await;
                completor_handle.set_v_char(args.remove(0));
                completor_handle.update_user_match(nvim.clone()).await;
                Ok::<_, anyhow::Error>(())
            });
        }
        "insert_leave" => {
            task::spawn(async move {
                info!("Clearing user match");
                completor.lock().await.user_match.write().await.clear();
                Ok::<_, anyhow::Error>(())
            });
        }
        "update_buffer_words" => {
            task::spawn(async move {
                let completor = completor.lock().await;
                let mut source = completor.sources.get("buffer_words").unwrap().lock().await;
                source.update(nvim).await?;
                Ok::<_, anyhow::Error>(())
            });
        }
        "add_lua_source" => {
            task::spawn(async move {
                let mut completor = completor.lock().await;
                let name: String = args.remove(0).try_unpack().unwrap();
                completor.register(&name.clone(), source::LuaFn::new(name));
                Ok::<_, anyhow::Error>(())
            });
        }
        _ => (),
    };
    Ok(())
}

pub async fn request() {}
