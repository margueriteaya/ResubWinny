//! Declares the in-process libmpv render route independently from caption rendering.

pub(crate) trait PreviewSurfaceRoute {
    fn id(&self) -> &'static str;
    fn requires_render_api(&self) -> bool;
    fn experimental(&self) -> bool;
}

struct LibMpvRenderSurface;

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

static RENDER: LibMpvRenderSurface = LibMpvRenderSurface;

pub(crate) fn declared_routes() -> [&'static dyn PreviewSurfaceRoute; 1] {
    [&RENDER]
}

pub(crate) fn capabilities(
    render_surface_ready: bool,
    native_embedding_supported: bool,
) -> Vec<crate::models::PreviewSurfaceCapability> {
    declared_routes()
        .into_iter()
        .map(|route| {
            let available = native_embedding_supported
                && (!route.requires_render_api() || render_surface_ready);
            let unavailable_reason_code = (!available).then(|| {
                if !native_embedding_supported {
                    "preview.platform_not_implemented".to_owned()
                } else {
                    "preview.render_surface_not_implemented".to_owned()
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
        let routes = capabilities(true, true);
        assert!(routes[0].available);
        assert!(!routes[0].experimental);
        assert!(routes[0].unavailable_reason_code.is_none());
        let missing_render = capabilities(false, true);
        assert_eq!(
            missing_render[0].unavailable_reason_code.as_deref(),
            Some("preview.render_surface_not_implemented")
        );
        let unsupported = capabilities(false, false);
        assert_eq!(
            unsupported[0].unavailable_reason_code.as_deref(),
            Some("preview.platform_not_implemented")
        );
        assert_eq!(declared_routes()[0].id(), "libmpv-render");
    }
}
