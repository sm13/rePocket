mod pocket;
mod pocketquery;
mod pocketitem;
mod articlehandler;
mod fshandler;
mod utils;

use pocket::Pocket;
use fshandler::FSHandler;
use pocketquery::QueryBuilder;

use reqwest::StatusCode;

#[tokio::main]
async fn main() {
    println!("ℹ Starting rePocket");

    // Initialize the "App"
    let mut pocket = Pocket::new();
    let mut fhandler = FSHandler::load();
    let _ = fhandler.mkdir_pocket().map_err(|_| { println!("ℹ Skipping, folder file already exists") });

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


    // OK, so this actually gets us articles, at least from some places, looks like the National
    // Geographic does something weird perhaps. In any case, the HTML has the URl for images, but
    // since the " are scaped they won't load. These are available in the json Pocket returns.
    // Presumably, we can download them using pocket, rename them, and fix the URL in the HTML.


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
                            let res = pocket.add_tag(id, vec!["repocket".to_string()]).await;
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

    fhandler.save_config();


    if env!("VERBOSITY") > "0" {
        println!("ℹ {:#?}", fhandler);
    }

    // Perhaps the next thing to do is to create a database with this info.
    // If the .db exists, then we load it from file, if it doesn't we created from the fields.
    // At the moment the .config file somewhat does this.
    //
    // Other than this, I guess I should try to get the complete resposnse. Which I guess
    // will provide, images, videos, etc. Try with a single article and do not get the videos!
    // Images are OK.
}
