package com.example.meliorsonus.ui.osmd

import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitView
import platform.WebKit.WKWebView
import platform.WebKit.WKWebViewConfiguration
import platform.WebKit.WKNavigationDelegateProtocol
import platform.darwin.NSObject
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.readValue
import platform.CoreGraphics.CGRectZero

@OptIn(ExperimentalForeignApi::class)
@Composable
actual fun OSMDWebView(
    xmlContent: String,
    mode: String,
    zoom: Float,
    currentMeasure: Int,
    pageNextCount: Int,
    pagePreviousCount: Int,
    isDragging: Boolean,
    modifier: Modifier
) {
    var isLoaded by remember { mutableStateOf(false) }
    
    val webView = remember {
        val config = WKWebViewConfiguration()
        WKWebView(frame = CGRectZero.readValue(), configuration = config)
    }

    val navigationDelegate = remember {
        object : NSObject(), WKNavigationDelegateProtocol {
            override fun webView(webView: WKWebView, didFinishNavigation: platform.WebKit.WKNavigation?) {
                isLoaded = true
            }
        }
    }

    LaunchedEffect(webView) {
        webView.navigationDelegate = navigationDelegate
        val path = platform.Foundation.NSBundle.mainBundle.pathForResource("osmd", "html", "osmd")
            ?: platform.Foundation.NSBundle.mainBundle.pathForResource("osmd", "html")
        if (path != null) {
            val url = platform.Foundation.NSURL.fileURLWithPath(path)
            webView.loadFileURL(url, allowingReadAccessToURL = url.URLByDeletingLastPathComponent!!)
        } else {
            // Log error
            println("Error: osmd.html not found in iOS Bundle")
        }
    }

    LaunchedEffect(isLoaded, xmlContent, mode) {
        if (isLoaded && xmlContent.isNotEmpty()) {
            val safeXml = xmlContent
                .replace("\\", "\\\\")  // \ → \\  (must come first)
                .replace("`", "\\`")    // ` → \`
            webView.evaluateJavaScript("initOSMD('$mode');", null)
            webView.evaluateJavaScript(
                "document.getElementById('xml-store').value = `$safeXml`; _doLoad();",
                null
            )
        }
    }

    LaunchedEffect(zoom) {
        if (isLoaded) {
            webView.evaluateJavaScript("setZoom($zoom);", null)
        }
    }

    LaunchedEffect(currentMeasure) {
        if (isLoaded) {
            webView.evaluateJavaScript("scrollToMeasure($currentMeasure);", null)
        }
    }

    // Tablet page navigation from Kotlin button presses
    LaunchedEffect(pageNextCount) {
        if (isLoaded && pageNextCount > 0) {
            webView.evaluateJavaScript("goToNextPage();", null)
        }
    }

    LaunchedEffect(pagePreviousCount) {
        if (isLoaded && pagePreviousCount > 0) {
            webView.evaluateJavaScript("goToPreviousPage();", null)
        }
    }

    UIKitView(
        factory = { webView },
        modifier = modifier,
        update = {}
    )
}
