use super::test_fixture::CommandTestWorld;

impl CommandTestWorld {
    pub(crate) fn with_bossbars(mut self, ids: &[&str]) -> Self {
        for id in ids {
            self.setup(&format!("bossbar add {id} \"Old name\""));
            self.setup(&format!("bossbar set {id} value 10"));
        }
        self
    }
}
