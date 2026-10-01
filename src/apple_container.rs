use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct BuildTransfer {
    pub metadata: HashMap<String, String>,
}

impl BuildTransfer {
    pub fn new(metadata: HashMap<String, String>) -> Self {
        Self { metadata }
    }

    pub fn stage(&self) -> Option<String> {
        self.metadata.get("stage").filter(|s| !s.is_empty()).cloned()
    }

    pub fn method(&self) -> Option<String> {
        self.metadata.get("method").filter(|s| !s.is_empty()).cloned()
    }

    pub fn include_patterns(&self) -> Option<Vec<String>> {
        self.metadata
            .get("include-patterns")
            .filter(|s| !s.is_empty())
            .map(|s| s.split(',').map(|p| p.to_string()).collect())
    }

    pub fn follow_paths(&self) -> Option<Vec<String>> {
        self.metadata
            .get("followpaths")
            .filter(|s| !s.is_empty())
            .map(|s| s.split(',').map(|p| p.to_string()).collect())
    }

    pub fn mode(&self) -> Option<String> {
        self.metadata.get("mode").cloned()
    }

    pub fn size(&self) -> Option<usize> {
        self.metadata
            .get("size")
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
    }

    pub fn offset(&self) -> Option<u64> {
        self.metadata
            .get("offset")
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
    }

    pub fn len(&self) -> Option<usize> {
        self.metadata
            .get("length")
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageTransfer {
    pub metadata: HashMap<String, String>,
}

impl ImageTransfer {
    pub fn new(metadata: HashMap<String, String>) -> Self {
        Self { metadata }
    }

    pub fn stage(&self) -> Option<String> {
        self.metadata.get("stage").cloned()
    }

    pub fn method(&self) -> Option<String> {
        self.metadata.get("method").cloned()
    }

    pub fn ref_name(&self) -> Option<String> {
        self.metadata.get("ref").cloned()
    }

    pub fn platform(&self) -> Option<String> {
        self.metadata.get("platform").cloned()
    }

    pub fn mode(&self) -> Option<String> {
        self.metadata.get("mode").cloned()
    }

    pub fn size(&self) -> Option<usize> {
        self.metadata.get("size").and_then(|s| s.parse().ok())
    }

    pub fn len(&self) -> Option<usize> {
        self.metadata.get("length").and_then(|s| s.parse().ok())
    }

    pub fn offset(&self) -> Option<u64> {
        self.metadata.get("offset").and_then(|s| s.parse().ok())
    }
}

#[derive(Debug, Clone)]
pub enum ServerStream {
    ImageTransfer(ImageTransfer),
    BuildTransfer(BuildTransfer),
    IO(Vec<u8>),
}

impl ServerStream {
    pub fn get_image_transfer(&self) -> Option<&ImageTransfer> {
        if let ServerStream::ImageTransfer(ref v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn get_build_transfer(&self) -> Option<&BuildTransfer> {
        if let ServerStream::BuildTransfer(ref v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn get_io(&self) -> Option<&[u8]> {
        if let ServerStream::IO(ref v) = self {
            Some(v)
        } else {
            None
        }
    }
}
