use crate::{
    error::{
        Error,
        ErrorKind,
    },
    runtime::{
        ModuleRef,
        ModulePath,
    },
    syntax::{
        Parser,
        Lexer,
        Program,
    },
};

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub struct ModuleLoader {
    root: PathBuf,
    cache: HashMap<PathBuf, ModuleRef>,

    #[cfg(test)]
    load_count: usize,
}

impl ModuleLoader {
    pub fn new(
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            root: root.into(),
            cache: HashMap::new(),

            #[cfg(test)]
            load_count: 0,
        }
    }

    #[cfg(test)]
    pub fn load_count(
        &self,
    ) -> usize {
        self.load_count
    }

    // =========================================================
    // Resolve a Novum module path to a physical .nv file.
    //
    // Example:
    //
    // tests.modules.a
    //
    // -> <root>/tests/modules/a.nv
    // =========================================================

    pub fn resolve(
    &self,
    module_path: &ModulePath,
    base_dir: Option<&Path>,
) -> Result<PathBuf, Error> {
    let mut candidates =
        Vec::new();

    // ---------------------------------------------------------
    // 1. Relative to importing module
    // ---------------------------------------------------------

    if let Some(base_dir) = base_dir {
        let mut path =
            base_dir.to_path_buf();

        for part in module_path.parts() {
            path.push(part);
        }

        path.set_extension("nv");

        candidates.push(path);
    }

    // ---------------------------------------------------------
    // 2. Relative to project root
    // ---------------------------------------------------------

    let mut root_path =
        self.root.clone();

    for part in module_path.parts() {
        root_path.push(part);
    }

    root_path.set_extension("nv");

    candidates.push(root_path);

    // ---------------------------------------------------------
    // 3. First existing candidate wins
    // ---------------------------------------------------------

    for path in candidates {
        if path.is_file() {
            return std::fs::canonicalize(
                &path
            )
            .map_err(|error| {
                Error::new(
                    ErrorKind::Import,
                    format!(
                        "failed to resolve module '{}': {}",
                        path.display(),
                        error
                    ),
                    None,
                )
            });
        }
    }

    Err(
        Error::new(
            ErrorKind::Import,
            format!(
                "module '{}' not found",
                module_path
            ),
            None,
        )
    )
}

    // =========================================================
    // Read and parse a module source.
    //
    // Evaluation is intentionally NOT performed here.
    // =========================================================

    pub fn load_program(
        &self,
        path: &Path,
    ) -> Result<Program, Error> {
        let source =
            fs::read_to_string(path)
                .map_err(|error| {
                    Error::new(
                        ErrorKind::Import,
                        format!(
                            "failed to read module '{}': {}",
                            path.display(),
                            error
                        ),
                        None,
                    )
                })?;

        let mut lexer =
            Lexer::new(&source);

        let tokens =
            lexer.lex()
                .map_err(|error| {
                    error
                })?;

        let mut parser =
            Parser::new(tokens);

        parser
            .parse()
            .map_err(|error| {
                error
            })
    }

    // =========================================================
    // Cache
    // =========================================================

    pub fn get_cached(
        &self,
        path: &Path,
    ) -> Option<ModuleRef> {
        self.cache.get(path).cloned()
    }

    pub fn cache(
        &mut self,
        path: PathBuf,
        module: ModuleRef,
    ) {
        self.cache.insert(
            path,
            module,
        );
    }
}