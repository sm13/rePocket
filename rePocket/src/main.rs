//
// Copyright (c) 2024 Damián Sánchez Moreno
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

mod pocket;
mod pocketquery;
mod pocketitem;
mod articlehandler;
mod fshandler;
mod utils;

use pocket::Pocket;
use fshandler::FSHandler;
use pocketquery::QueryBuilder;

use std::path::Path;
use reqwest::StatusCode;
use tokio::sync::mpsc::{channel, Receiver};
use notify_debouncer_mini::{
    notify::*,
    new_debouncer,
    Debouncer,
    DebouncedEvent,
    DebouncedEventKind,
};


#[tokio::main]
async fn main() {
    println!("ℹ Starting rePocket");

    // Initialize the "App"
    let mut pocket = Pocket::new();
    let mut fhandler = FSHandler::load();
    let _ = fhandler.mkdir_pocket().map_err(|_| { println!("ℹ Skipping, folder file already exists") });
    // Path to the Pocket/Sync folder.
    let path = fshandler::XOCHITL_ROOT.to_string();


    if let Err(e) = async_watch(path, &mut pocket, &mut fhandler).await {
        println!("🚨 Error: {:?}", e)
    }
}



// The async watcher uses the Debouncer version of Notify to filter-out multiple events for the
// same Path. This seems to work better, and produces less useless iterations than the
// alternative.
fn async_watcher() -> notify::Result<(Debouncer<RecommendedWatcher>, Receiver<notify::Result<Vec<DebouncedEvent>>>)> {
    let (tx, rx) = channel(1);
    let thandle = tokio::runtime::Handle::current();

    // Automatically select the best implementation for the underlying platform.
    let debouncer = new_debouncer(std::time::Duration::from_secs(1),
        move |res| {
            thandle.block_on(async {
                tx.send(res).await.unwrap();
            })
        }
    )?;

    Ok((debouncer, rx))
}


async fn async_watch<P: AsRef<Path>>(path: P, pocket: &mut Pocket, fhandler: &mut FSHandler) -> notify::Result<()> {
    let wfname = path.as_ref().join(fhandler.sync_uuid_string() + ".metadata");
    let (mut debouncer, mut rx) = async_watcher().expect("Could not start notify");

    // Add the path (file, in this case to be watched)
    debouncer.watcher().watch(path.as_ref(), RecursiveMode::NonRecursive).unwrap();

    while let Some(res) = rx.recv().await {
        match res {
            Ok(events) => {

                for event in events {
                    if event.path == wfname && event.kind == DebouncedEventKind::Any {
                        println!("ℹ Found syncing event: {:?}", event);

                        // This should be the entry point for the watching changes to the Sync Folder.
                        let since = fhandler.last_query_ts();

                        let complete_query = QueryBuilder::default()
                            .set_state("Unread")
                            .set_favorite(0)
                            //.set_tag("pdf")
                            //.set_content_type("Article")
                            .set_sort("Newest")
                            .set_detail_type("Complete")
                            //.set_search("learn")
                            //.set_domain(".com")
                            .set_since(since)
                            .set_count(10)
                            .set_offset(0)
                            .set_total(1)
                            .build();

                        let res = pocket.retrieve(&complete_query.unwrap()).await;

                        // Send the result for processing, that is, create a list of PocketItems.
                        //
                        // This are the higher-level fields for the response.
                        //      "maxActions":30,
                        //      "cachetype":"db",
                        //      "status":1,
                        //      "error":null,
                        //      "complete":1,
                        //      "since":1729763686,
                        //      "list": { // This is the list object referred to in the documentation with as many
                        //          id : { response fields as requested up to a max of 30 }
                        //
                        // The value "since" should be stored so as to pass it again on the next _efficient_ request.
                        //
                        match res {
                            Ok(val) => {
                                pocket.init(val).await;
                                fhandler.set_last_query_ts(pocket.since());
                            },
                            Err(e) => println!("🚨 Error {e}"),
                        };


                        for item in pocket.iter() {
                            println!("ℹ Working on item id {:?} with URL\n  ..{:?}", item.get_resolved_id(), item.get_resolved_url());
                            fhandler.new_article(&item).await;
                        }

                        // Archive all the items in the Read folder
                        let ids : Vec<u64> = fhandler.read_ids().collect();

                        if !ids.is_empty() {
                            let res = pocket.archive(ids.clone()).await;

                            // TODO: When proper error handling is implemented, this could be absorved by archive() and
                            // dealt with over there. Returning error unless we get a 200 response.
                            match res {
                                Ok(val) => {
                                    let status = val.status();
                                    match status.clone() {
                                        StatusCode::OK => {
                                            // Tag all items
                                            for id in fhandler.read_ids() {
                                                println!("ℹ Tagging item id {:?} with tag 'repocket'", id);
                                                let _res = pocket.add_tag(id, vec!["repocket".to_string()]).await;
                                            }

                                            // Remove all items form the read_items entry in the FSHandler.
                                            fhandler.clear_read();
                                        },
                                        _ => println!("🚨 Error, archive() returned with status {:?}", status),
                                    }
                                },
                                Err(e) =>  println!("🚨 Error {e}"),
                            }
                        }


                        // Save and reload fhandler.
                        fhandler.save_config();

                        let fhandler = FSHandler::load();

                        println!("ℹ Unwatching the Sync folder while Xochitl restarts");
                        let _ = debouncer.watcher().unwatch(path.as_ref());

                        if cfg!(target_abi = "eabihf") {
                            let cmd = std::thread::spawn(move || {
                                std::process::Command::new("systemctl")
                                    .arg("restart")
                                    .arg("xochitl")
                                    .output()
                                    .expect("Could not restart Xochitl");

                                println!(" .. sleeping for some empirical number of seconds during restart");

                                std::thread::sleep(std::time::Duration::new(30, 0));
                            });

                            let _result = cmd.join().unwrap();
                        } else {
                            println!("ℹ In the remarkable we'd be restarting Xochitl");
                        }

                        println!("ℹ Watching the Sync folder again");
                        debouncer.watcher().watch(path.as_ref(), RecursiveMode::NonRecursive).unwrap();

                        // This could be taken out and logged once, at the end, for instance.
                        println!("ℹ {:#?}", fhandler);

                        // Break out of the loop early
                        break;
                    }
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

