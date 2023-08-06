use semver::Version;

pub trait VersionClamp: Sized {
    fn clamp_version(&mut self, version: &Version);

    fn into_clamp_version(mut self, version: &Version) -> Self {
        self.clamp_version(version);
        self
    }
}
