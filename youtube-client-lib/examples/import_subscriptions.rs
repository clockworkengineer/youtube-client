//! # Example: Import Subscriptions from Google Takeout, OPML, and NewPipe
//!
//! Demonstrates:
//! - Importing subscriptions offline from OPML XML, Google Takeout CSV, and NewPipe JSON.
//! - Exporting subscriptions to OPML.
//!
//! Run with:
//! ```bash
//! cargo run --example import_subscriptions
//! ```

use youtube_client_lib::{
    export_subscriptions_to_opml, import_subscriptions_auto_detect, import_subscriptions_from_opml,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YouTube Client: Subscription Import/Export Example ===");

    // Sample OPML XML document
    let sample_opml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.1">
    <head>
        <title>YouTube Subscriptions</title>
    </head>
    <body>
        <outline text="YouTube Subscriptions" title="YouTube Subscriptions">
            <outline text="3Blue1Brown" title="3Blue1Brown" type="rss"
                xmlUrl="https://www.youtube.com/feeds/videos.xml?channel_id=UCYO_jab_esuFRV4b17AJtAw"/>
            <outline text="Veritasium" title="Veritasium" type="rss"
                xmlUrl="https://www.youtube.com/feeds/videos.xml?channel_id=UCHnyfMqiRRG1u-2MsSQLbXA"/>
        </outline>
    </body>
</opml>"#;

    println!("Importing sample OPML document...");
    let subscriptions = import_subscriptions_from_opml(sample_opml)?;
    println!("Parsed {} channels:", subscriptions.len());
    for sub in &subscriptions {
        println!("  - Channel: {:<15} | ID: {}", sub.channel_title, sub.channel_id);
    }

    // Export back to OPML
    let exported_opml = export_subscriptions_to_opml(&subscriptions);
    println!("\nExported back to valid OPML ({} bytes).", exported_opml.len());

    // Auto-detect format from CSV content
    let sample_csv = "Channel Id,Channel Url,Channel Title\nUC_test_1,http://youtube.com/channel/UC_test_1,Test Channel";
    let detected = import_subscriptions_auto_detect(sample_csv, None)?;
    println!("\nAuto-detected and imported {} channels from CSV format.", detected.len());

    Ok(())
}
