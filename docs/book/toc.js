// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="spacer"></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="quick_start.html">Quick Start</a></li><li class="chapter-item expanded "><a href="installation.html">Installation</a></li><li class="chapter-item expanded affix "><li class="part-title">Architecture</li><li class="chapter-item expanded "><a href="architecture/overview.html">Overview</a></li><li class="chapter-item expanded "><a href="architecture/data_flow.html">Data Flow</a></li><li class="chapter-item expanded "><a href="architecture/storage.html">Storage Layer</a></li><li class="chapter-item expanded affix "><li class="part-title">Core Concepts</li><li class="chapter-item expanded "><a href="concepts/entities.html">Entities</a></li><li class="chapter-item expanded "><a href="concepts/feature_views.html">Feature Views</a></li><li class="chapter-item expanded "><a href="concepts/data_sources.html">Data Sources</a></li><li class="chapter-item expanded "><a href="concepts/feature_services.html">Feature Services</a></li><li class="chapter-item expanded "><a href="concepts/entity_keys.html">Entity Keys</a></li><li class="chapter-item expanded "><a href="concepts/value_types.html">Value Types</a></li><li class="chapter-item expanded affix "><li class="part-title">Rust SDK</li><li class="chapter-item expanded "><a href="rust/overview.html">Crate Overview</a></li><li class="chapter-item expanded "><a href="rust/ofs_core.html">ofs-core</a></li><li class="chapter-item expanded "><a href="rust/ofs_registry.html">ofs-registry</a></li><li class="chapter-item expanded "><a href="rust/ofs_offline_store.html">ofs-offline-store</a></li><li class="chapter-item expanded "><a href="rust/ofs_online_store.html">ofs-online-store</a></li><li class="chapter-item expanded "><a href="rust/ofs_materialization.html">ofs-materialization</a></li><li class="chapter-item expanded "><a href="rust/ofs_proto.html">ofs-proto</a></li><li class="chapter-item expanded affix "><li class="part-title">Python SDK</li><li class="chapter-item expanded "><a href="python/getting_started.html">Getting Started</a></li><li class="chapter-item expanded "><a href="python/feature_store.html">FeatureStore API</a></li><li class="chapter-item expanded "><a href="python/types.html">Type Reference</a></li><li class="chapter-item expanded "><a href="python/examples.html">Examples</a></li><li class="chapter-item expanded affix "><li class="part-title">CLI Reference</li><li class="chapter-item expanded "><a href="cli/overview.html">Overview</a></li><li class="chapter-item expanded "><a href="cli/commands.html">Commands</a></li><li class="chapter-item expanded affix "><li class="part-title">MCP Server</li><li class="chapter-item expanded "><a href="mcp/overview.html">Overview</a></li><li class="chapter-item expanded "><a href="mcp/protocol.html">Protocol</a></li><li class="chapter-item expanded affix "><li class="part-title">Operations</li><li class="chapter-item expanded "><a href="operations/configuration.html">Configuration</a></li><li class="chapter-item expanded "><a href="operations/deployment.html">Deployment</a></li><li class="chapter-item expanded "><a href="operations/monitoring.html">Monitoring</a></li><li class="chapter-item expanded affix "><li class="part-title">Development</li><li class="chapter-item expanded "><a href="development/contributing.html">Contributing</a></li><li class="chapter-item expanded "><a href="development/building.html">Building from Source</a></li><li class="chapter-item expanded "><a href="development/testing.html">Testing</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
