//! Declares native video-surface routes independently from caption rendering.
//!
//! The Windows render route owns a WGL surface and complete libmpv render loop.
//! A client-overlay route remains an explicit per-source fallback when the
//! runtime lacks that API or native render initialization fails.

pub(crate) trait PreviewSurfaceRoute {
    fn id(&self) -> &'static str;
    fn requires_render_api(&self) -> bool;
    fn experimental(&self) -> bool;
}

struct LibMpvClientOverlaySurface;
struct LibMpvRenderSurface;

impl PreviewSurfaceRoute for LibMpvClientOverlaySurface {
    fn id(&self) -> &'static str {
        "libmpv-client-overlay"
    }

    fn requires_render_api(&self) -> bool {
        false
    }

    fn experimental(&self) -> bool {
        false
    }
}

impl PreviewSurfaceRoute for LibMpvRenderSurface {
    fn id(&self) -> &'static str {
        "libmpv-render"
    }

    fn requires_render_api(&self) -> bool {
        true
    }

    fn experimental(&self) -> bool {
        false
    }
}

static CLIENT_OVERLAY: LibMpvClientOverlaySurface = LibMpvClientOverlaySurface;
static RENDER: LibMpvRenderSurface = LibMpvRenderSurface;

pub(crate) fn declared_routes() -> [&'static dyn PreviewSurfaceRoute; 2] {
    [&CLIENT_OVERLAY, &RENDER]
}

pub(crate) fn capabilities(
    client_runtime_ready: bool,
    render_surface_ready: bool,
    native_embedding_supported: bool,
) -> Vec<crate::models::PreviewSurfaceCapability> {
    declared_routes()
        .into_iter()
        .map(|route| {
            let available = native_embedding_supported
                && if route.requires_render_api() {
                    render_surface_ready
                } else {
                    client_runtime_ready
                };
            let unavailable_reason_code = (!available).then(|| {
                if !native_embedding_supported {
                    "preview.platform_not_implemented".to_owned()
                } else if route.requires_render_api() {
                    "preview.render_surface_not_implemented".to_owned()
                } else {
                    "preview.libmpv_runtime_unavailable".to_owned()
                }
            });
            crate::models::PreviewSurfaceCapability {
                id: route.id().to_owned(),
                available,
                experimental: route.experimental(),
                unavailable_reason_code,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{capabilities, declared_routes};

    #[test]
    fn render_surface_is_available_only_when_the_platform_runtime_is_ready() {
        let routes = capabilities(true, true, true);
        assert!(routes[0].available);
        assert!(!routes[0].experimental);
        assert!(routes[1].available);
        assert!(!routes[1].experimental);
        assert!(routes[1].unavailable_reason_code.is_none());
        let missing_render = capabilities(true, false, true);
        assert_eq!(
            missing_render[1].unavailable_reason_code.as_deref(),
            Some("preview.render_surface_not_implemented")
        );
        let unsupported = capabilities(true, false, false);
        assert_eq!(
            unsupported[0].unavailable_reason_code.as_deref(),
            Some("preview.platform_not_implemented")
        );
        assert_eq!(declared_routes()[1].id(), "libmpv-render");
    }
}
