use std::collections::HashMap;

use lazy_static::lazy_static;

pub struct SourceEntry<'a> {
    pub id: usize,
    pub url_prefix: &'a str,
}

macro_rules! environment {
    ($key: expr) => {
        std::env::var($key).expect(&format!("not set environment variable: {}", $key))
    };
}

lazy_static! {
    pub static ref SOURCES: HashMap<&'static str, SourceEntry<'static>> = {
        [
            ("BBC NEWS CHANNEL HD", SourceEntry { id: 0, url_prefix: "https://vs-cmaf-push-uk.live.fastly.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_news_channel_hd/"}),
            ("BBC WORLD NEWS AMERICA HD", SourceEntry { id: 1, url_prefix: "https://vs-cmaf-pushb-ntham-gcomm-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_world_news_north_america/"}),
            ("BBC ONE HD", SourceEntry { id: 2, url_prefix: "https://vs-cmaf-push-uk.live.fastly.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_hd/"}),
            ("BBC ONE WALES HD", SourceEntry { id: 3, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_wales_hd/"}),
            ("BBC ONE SCOTLAND HD", SourceEntry { id: 4, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_scotland_hd/"}),
            ("BBC ONE NORTHERN IRELAND HD", SourceEntry { id: 5, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_northern_ireland_hd/"}),
            ("BBC ONE CHANNEL ISLANDS HD", SourceEntry { id: 6, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_channel_islands/"}),
            ("BBC ONE EAST HD", SourceEntry { id: 7, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_east/"}),
            ("BBC ONE EAST MIDLANDS HD", SourceEntry { id: 8, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_east_midlands/"}),
            ("BBC ONE EAST YORKSHIRE & LINCONSHIRE HD", SourceEntry { id: 9, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_east_yorkshire/"}),
            ("BBC ONE LONDON HD", SourceEntry { id: 10, url_prefix: "https://vs-cmaf-push-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_london/"}),
            ("BBC ONE NORTH EAST HD", SourceEntry { id: 11, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_north_east/"}),
            ("BBC ONE NORTH WEST HD", SourceEntry { id: 12, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_north_west/"}),
            ("BBC ONE SOUTH HD", SourceEntry { id: 13, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_south/"}),
            ("BBC ONE SOUTH EAST HD", SourceEntry { id: 14, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_south_east/"}),
            ("BBC ONE SOUTH WEST HD", SourceEntry { id: 15, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_south_west/"}),
            ("BBC ONE WEST HD", SourceEntry { id: 16, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_west/"}),
            ("BBC ONE WEST MIDLANDS HD", SourceEntry { id: 17, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_one_west_midlands/"}),
            ("BBC ONE YORKSHIRE HD", SourceEntry { id: 18, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_one_yorks/"}),
            ("BBC TWO HD", SourceEntry { id: 19, url_prefix: "https://vs-cmaf-push-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_two_hd/"}),
            ("BBC TWO NORTHERN IRELAND HD", SourceEntry { id: 20, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_two_northern_ireland_hd/"}),
            ("BBC TWO WALES DIGITAL", SourceEntry { id: 21, url_prefix: "https://vs-cmaf-pushb-uk.live.fastly.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_two_wales_digital/"}),
            ("BBC THREE HD", SourceEntry { id: 22, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_three_hd/"}),
            ("BBC FOUR HD", SourceEntry { id: 23, url_prefix: "https://vs-cmaf-pushb-uk.live.cf.md.bbci.co.uk/x=4/i=urn:bbc:pips:service:bbc_four_hd/"}),
            ("CBBC HD", SourceEntry { id: 24, url_prefix: "https://b2-hobir-sky.live.bidi.net.uk/vs-cmaf-pushb-uk/x=4/i=urn:bbc:pips:service:cbbc_hd/"}),
            ("CBEEBIES HD", SourceEntry { id: 25, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:cbeebies_hd/"}),
            ("BBC SCOTLAND HD", SourceEntry { id: 26, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_scotland_hd/"}),
            ("BBC PARLIAMENT", SourceEntry { id: 27, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_parliament/"}),
            ("BBC ALBA", SourceEntry { id: 28, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_alba/"}),
            ("S4C", SourceEntry { id: 29, url_prefix: "https://vs-cmaf-pushb-uk-live.akamaized.net/x=4/i=urn:bbc:pips:service:s4cpbs/"}),
        ].into_iter().collect()
    };
    pub static ref DATABASE_URL: String = environment!("DATABASE_URL");
    pub static ref WEBDAV_URL: String = environment!("WEBDAV_URL");
    pub static ref WEBDAV_PASSWORD: String = environment!("WEBDAV_PASSWORD");
    pub static ref WEBDAV_USERNAME: String = environment!("WEBDAV_USERNAME");
}
