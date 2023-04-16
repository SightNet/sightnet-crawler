use url::Url;

pub fn normalize_url(url: &mut Url) {
    if url.fragment().is_some() {
        url.set_fragment(None);
    }
    if url.query().is_some() {
        url.set_query(None);
    }
}