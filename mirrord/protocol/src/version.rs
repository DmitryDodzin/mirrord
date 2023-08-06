use semver::Version;

pub trait VersionClamp: Sized {
    fn min_version(self, _version: &Version) -> Self {
        self
    }
}
