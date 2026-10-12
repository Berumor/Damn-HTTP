//! Keeping a request's URL, path variables and query table consistent.
//!
//! The URL bar shows one string; the request stores three things: the URL
//! template without a query string, the path variable table and the query
//! table. These methods translate between the two, using the shared URL
//! parser (D-028).

use super::request::{PathParam, QueryParam, Request};
use crate::url::{analyze, join_query, split_query};

impl Request {
    /// The names of the `:pathVariables` in the URL, in order of first appearance.
    pub fn path_variable_names(&self) -> Vec<String> {
        analyze(&self.url)
            .path_variable_names()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Path variable entries whose name is not in the URL any more. They are
    /// shown as such after loading and dropped by the next save (D-028).
    pub fn orphan_path_params(&self) -> Vec<&str> {
        let names = self.path_variable_names();
        self.path_params
            .iter()
            .map(|param| param.name.as_str())
            .filter(|name| !names.iter().any(|n| n == name))
            .collect()
    }

    /// Makes the path variable table match the URL: one entry per variable,
    /// in URL order, keeping existing values and descriptions. Entries whose
    /// name is not in the URL are kept at the end when `keep_orphans` is
    /// true, and dropped otherwise.
    pub fn align_path_params(&mut self, keep_orphans: bool) {
        let mut old = std::mem::take(&mut self.path_params);
        for name in self.path_variable_names() {
            let entry = match old.iter().position(|param| param.name == name) {
                Some(index) => old.remove(index),
                None => PathParam {
                    name,
                    value: String::new(),
                    description: String::new(),
                },
            };
            self.path_params.push(entry);
        }
        if keep_orphans {
            self.path_params.append(&mut old);
        }
    }

    /// The text for the URL bar: the URL with the enabled query parameters.
    pub fn url_bar_text(&self) -> String {
        join_query(
            &self.url,
            self.query
                .iter()
                .filter(|param| param.enabled)
                .map(|param| (param.name.as_str(), param.value.as_str())),
        )
    }

    /// Applies what the user typed or pasted in the URL bar.
    ///
    /// - The query string is taken out of the URL and becomes the enabled
    ///   rows of the query table. Disabled rows stay where they are; a row
    ///   keeps its description when its name is unchanged.
    /// - The path variable table follows the URL. When exactly one variable
    ///   disappears and one appears, it is a rename and the value and
    ///   description move with it (unless an orphan already has the new name). Other entries no longer in the URL are
    ///   kept as orphans until the next save, so retyping a name brings its
    ///   value back.
    pub fn set_url_bar_text(&mut self, text: &str) {
        let split = split_query(text.trim());

        let old_names = self.path_variable_names();
        self.url = split.url;
        let new_names = self.path_variable_names();
        let removed: Vec<&String> = old_names
            .iter()
            .filter(|n| !new_names.contains(n))
            .collect();
        let added: Vec<&String> = new_names
            .iter()
            .filter(|n| !old_names.contains(n))
            .collect();
        if let ([removed], [added]) = (removed.as_slice(), added.as_slice())
            // An orphan with the new name means the user typed an old name
            // again: its own value comes back, so this is not a rename.
            && !self.path_params.iter().any(|param| &param.name == *added)
            && let Some(param) = self.path_params.iter_mut().find(|p| &p.name == *removed)
        {
            param.name = (*added).clone();
        }
        self.align_path_params(true);

        let mut typed = split.query.into_iter();
        let mut rows = Vec::with_capacity(self.query.len());
        for row in std::mem::take(&mut self.query) {
            if !row.enabled {
                rows.push(row);
            } else if let Some((name, value)) = typed.next() {
                let description = if row.name == name {
                    row.description
                } else {
                    String::new()
                };
                rows.push(QueryParam {
                    name,
                    value,
                    enabled: true,
                    description,
                });
            }
        }
        rows.extend(typed.map(|(name, value)| QueryParam {
            name,
            value,
            enabled: true,
            description: String::new(),
        }));
        self.query = rows;
    }

    /// Moves a query string that is inside `url` into the query table, as
    /// enabled rows at the end. `url` is not supposed to hold one; this
    /// repairs a file edited by hand, and [`Request::split`] calls it so a
    /// literal in the URL's query can never be committed.
    pub fn move_query_out_of_url(&mut self) {
        if analyze(&self.url).query.is_none() {
            return;
        }
        let split = split_query(&self.url);
        self.url = split.url;
        self.query
            .extend(split.query.into_iter().map(|(name, value)| QueryParam {
                name,
                value,
                enabled: true,
                description: String::new(),
            }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{OrderKey, ValidationError};

    fn request() -> Request {
        Request::new("Stations", OrderKey::first())
    }

    fn row(name: &str, value: &str, enabled: bool, description: &str) -> QueryParam {
        QueryParam {
            name: name.to_owned(),
            value: value.to_owned(),
            enabled,
            description: description.to_owned(),
        }
    }

    fn path(name: &str, value: &str) -> PathParam {
        PathParam {
            name: name.to_owned(),
            value: value.to_owned(),
            description: String::new(),
        }
    }

    /// The maintainer's example, end to end through the model.
    #[test]
    fn a_pasted_url_never_reaches_the_committed_part_with_its_values() {
        let mut request = request();
        request.set_url_bar_text("  https://some.service/:country/stations/?id=1234\n");
        request.path_params[0].value = "it".to_owned();

        assert_eq!(request.url, "https://some.service/:country/stations/");
        assert_eq!(request.query, [row("id", "1234", true, "")]);
        assert_eq!(request.path_params, [path("country", "it")]);
        assert_eq!(
            request.url_bar_text(),
            "https://some.service/:country/stations/?id=1234"
        );

        let (committed, local) = request.split();
        let committed = committed.as_request();
        assert_eq!(committed.url, "https://some.service/:country/stations/");
        assert_eq!(committed.query, [row("id", "", true, "")]);
        assert_eq!(committed.path_params, []);
        assert_eq!(local.query[0].value, "1234");
        assert_eq!(local.path_params["country"], "it");
    }

    #[test]
    fn a_query_left_in_the_url_is_moved_out_by_split() {
        let mut request = request();
        request.url = "https://x.test/a?id=1234&t={{token}}#top".to_owned();
        assert_eq!(request.validate(), [ValidationError::QueryInUrl]);

        let (committed, local) = request.split();

        let committed = committed.as_request();
        assert_eq!(committed.url, "https://x.test/a#top");
        assert_eq!(committed.validate(), []);
        assert_eq!(
            committed.query,
            [row("id", "", true, ""), row("t", "{{token}}", true, "")]
        );
        assert_eq!(local.query.len(), 1);
        assert_eq!(local.query[0].value, "1234");
    }

    #[test]
    fn editing_the_url_bar_rewrites_enabled_rows_and_keeps_disabled_ones() {
        let mut request = request();
        request.url = "/search".to_owned();
        request.query = vec![
            row("q", "cats", true, "Search text"),
            row("debug", "1", false, "Only for testing"),
            row("page", "2", true, "Page number"),
        ];
        assert_eq!(request.url_bar_text(), "/search?q=cats&page=2");

        request.set_url_bar_text("/search?q=dogs&size=10&sort=asc");
        assert_eq!(
            request.query,
            [
                row("q", "dogs", true, "Search text"),
                row("debug", "1", false, "Only for testing"),
                row("size", "10", true, ""),
                row("sort", "asc", true, ""),
            ]
        );

        request.set_url_bar_text("/search");
        assert_eq!(
            request.query,
            [row("debug", "1", false, "Only for testing")]
        );
        assert_eq!(request.url_bar_text(), "/search");
    }

    #[test]
    fn the_url_bar_round_trips() {
        let mut request = request();
        for text in [
            "https://some.service/:country/stations/?id=1234",
            "{{baseUrl}}/pets?tag={{tag}}&limit=20#list",
            "/plain",
            "/a?flag&x=1",
        ] {
            request.set_url_bar_text(text);
            assert_eq!(request.url_bar_text(), text);
        }
    }

    #[test]
    fn the_path_variable_table_follows_the_url() {
        let mut request = request();
        request.set_url_bar_text("/:country/stations/:id");
        assert_eq!(request.path_params, [path("country", ""), path("id", "")]);
        request.path_params[0].value = "it".to_owned();
        request.path_params[1].value = "42".to_owned();

        request.set_url_bar_text("/v2/:id/in/:country");
        assert_eq!(
            request.path_params,
            [path("id", "42"), path("country", "it")]
        );
        assert_eq!(request.orphan_path_params(), Vec::<&str>::new());
    }

    #[test]
    fn renaming_a_variable_in_the_url_moves_its_value() {
        let mut request = request();
        request.set_url_bar_text("/:country/stations/:id");
        request.path_params[0].value = "it".to_owned();
        request.path_params[0].description = "ISO code".to_owned();
        request.path_params[1].value = "42".to_owned();

        request.set_url_bar_text("/:nation/stations/:id");

        assert_eq!(request.path_params[0].name, "nation");
        assert_eq!(request.path_params[0].value, "it");
        assert_eq!(request.path_params[0].description, "ISO code");
        assert_eq!(request.path_params[1], path("id", "42"));
        assert_eq!(request.orphan_path_params(), Vec::<&str>::new());
    }

    #[test]
    fn removed_variables_become_orphans_until_aligned_for_saving() {
        let mut request = request();
        request.set_url_bar_text("/:country/stations/:id");
        request.path_params[0].value = "it".to_owned();
        request.path_params[1].value = "42".to_owned();

        // Two names change at once: not a rename.
        request.set_url_bar_text("/:a/stations/:b");
        assert_eq!(request.orphan_path_params(), ["country", "id"]);
        assert_eq!(request.path_params.len(), 4);

        // Typing the old name again brings the value back.
        request.set_url_bar_text("/:a/stations/:id");
        assert_eq!(request.path_params[1], path("id", "42"));
        assert_eq!(request.orphan_path_params(), ["b", "country"]);

        request.align_path_params(false);
        assert_eq!(request.path_params, [path("a", ""), path("id", "42")]);
    }

    #[test]
    fn deleting_one_variable_is_not_a_rename() {
        let mut request = request();
        request.set_url_bar_text("/:country/stations/:id");
        request.path_params[1].value = "42".to_owned();

        request.set_url_bar_text("/:country/stations");

        assert_eq!(request.path_params, [path("country", ""), path("id", "42")]);
        assert_eq!(request.orphan_path_params(), ["id"]);
    }
}
