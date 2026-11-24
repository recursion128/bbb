use super::test_fixture::CommandTestWorld;
use bbb_server_datapack::function::FunctionLibrary;

impl CommandTestWorld {
    pub(crate) fn with_functions(mut self, library: &FunctionLibrary) -> Self {
        self.world.replace_function_library(library);
        self
    }
}
