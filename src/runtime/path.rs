use std::{
    fmt,
    path::{Path, PathBuf},
    rc::Rc,
};

pub type PathRef = Rc<PathValue>;

#[derive(Clone, PartialEq, Eq)]
pub struct PathValue {
    path: PathBuf,
}

impl PathValue {
    pub fn new(
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            path: path.into(),
        }
    }

    pub fn as_path(
        &self,
    ) -> &Path {
        &self.path
    }

    pub fn to_path_buf(
        &self,
    ) -> PathBuf {
        self.path.clone()
    }

    pub fn to_string_lossy(
        &self,
    ) -> String {
        self.path
            .to_string_lossy()
            .into_owned()
    }

    pub fn name(
        &self,
    ) -> Option<String> {
        self.path
            .file_name()
            .map(|name| {
                name.to_string_lossy()
                    .into_owned()
            })
    }

    pub fn extension(
        &self,
    ) -> Option<String> {
        self.path
            .extension()
            .map(|extension| {
                extension
                    .to_string_lossy()
                    .into_owned()
            })
    }

    pub fn stem(
        &self,
    ) -> Option<String> {
        self.path
            .file_stem()
            .map(|stem| {
                stem.to_string_lossy()
                    .into_owned()
            })
    }

    pub fn parent(
        &self,
    ) -> Option<Self> {
        self.path
            .parent()
            .map(Self::new)
    }

    pub fn join<P>(
        &self,
        child: P,
    ) -> Self
    where
        P: AsRef<Path>,
    {
        Self::new(
            self.path.join(child)
        )
    }

    pub fn exists(
        &self,
    ) -> bool {
        self.path.exists()
    }

    pub fn is_file(
        &self,
    ) -> bool {
        self.path.is_file()
    }

    pub fn is_dir(
        &self,
    ) -> bool {
        self.path.is_dir()
    }
}

impl fmt::Debug for PathValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_tuple("Path")
            .field(
                &self.to_string_lossy()
            )
            .finish()
    }
}

impl fmt::Display for PathValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}",
            self.path.display()
        )
    }
}