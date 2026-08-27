use crate::{
    error::{
        Error,
        ErrorKind,
        Result,
    },
    syntax::{
        Lexer,
        Parser,
        Program,
    },
    vm::{
        Chunk,
        Compiler,
    },
};

use std::{
    collections::HashMap,
    fs,
    path::{
        Path,
        PathBuf,
    },
    rc::Rc,
};

pub struct ModuleLoader {
    root: PathBuf,
    chunk_cache: HashMap<PathBuf, Rc<Chunk>>,
}

impl ModuleLoader {
    pub fn new(
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            root: root.into(),
            chunk_cache:
                HashMap::new(),
        }
    }

    pub fn resolve(
        &self,
        module_path: &crate::runtime::ModulePath,
        importing_file: Option<&Path>,
    ) -> Result<PathBuf> {
        let mut candidates =
            Vec::new();

        if let Some(file) =
            importing_file
        {
            if let Some(parent) =
                file.parent()
            {
                let mut path =
                    parent.to_path_buf();

                for part
                    in module_path.parts()
                {
                    path.push(part);
                }

                path.set_extension("nv");

                candidates.push(path);
            }
        }

        let mut root =
            self.root.clone();

        for part
            in module_path.parts()
        {
            root.push(part);
        }

        root.set_extension("nv");

        candidates.push(root);

        for candidate in
            candidates
        {
            if candidate.is_file() {
                return fs::canonicalize(
                    &candidate
                )
                .map_err(|error| {
                    Error::new(
                        ErrorKind::Import,
                        format!(
                            "failed to canonicalize module '{}': {}",
                            candidate.display(),
                            error,
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

    pub fn load_chunk(
        &mut self,
        path: &Path,
    ) -> Result<Rc<Chunk>> {
        let path =
            fs::canonicalize(path)
                .map_err(|error| {
                    Error::new(
                        ErrorKind::Import,
                        format!(
                            "failed to resolve module '{}': {}",
                            path.display(),
                            error,
                        ),
                        None,
                    )
                })?;

        if let Some(chunk) =
            self.chunk_cache
                .get(&path)
        {
            return Ok(
                chunk.clone()
            );
        }

        let source =
            fs::read_to_string(
                &path
            )
            .map_err(|error| {
                Error::new(
                    ErrorKind::Import,
                    format!(
                        "failed to read module '{}': {}",
                        path.display(),
                        error,
                    ),
                    None,
                )
            })?;

        let tokens =
            Lexer::new(&source)
                .lex()?;

        let mut parser =
            Parser::new(tokens);

        let program:
            Program =
            parser.parse()?;

        let chunk =
            Compiler::new()
                .compile(&program)?;

        let chunk =
            Rc::new(chunk);

        self.chunk_cache.insert(
            path,
            chunk.clone(),
        );

        Ok(chunk)
    }
}