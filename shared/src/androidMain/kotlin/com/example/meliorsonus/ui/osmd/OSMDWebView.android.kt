package com.example.meliorsonus.ui.osmd

import android.annotation.SuppressLint
import android.util.Base64
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.viewinterop.AndroidView

private const val OSMD_HTML = """<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <!-- Load Open Sheet Music Display from CDN -->
    <script src="https://unpkg.com/opensheetmusicdisplay/build/opensheetmusicdisplay.min.js"></script>
    <style>
        /* Keeping only your original CSS */
        body { margin: 0; overflow: hidden; background-color: #ffffff; }
        #osmd-container { width: 100%; height: 100%; display: flex; gap: 0; }
        #osmd-container > div {
            flex: 0 0 100vw !important; /* Do not shrink, take exactly 100% of viewport width */
            width: 100vw !important;
            max-width: 100vw !important;
            overflow: hidden !important; /* Chop off anything trying to bleed into the next page */
            box-sizing: border-box;
        }
        #loading-overlay {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background-color: #ffffff;
            display: flex;
            justify-content: center;
            align-items: center;
            z-index: 1000; /* Keeps it on top of everything */
            font-family: sans-serif;
            font-size: 18px;
        }
    </style>
</head>
<body>
    <div id="loading-overlay">Rendering Music...</div>

    <!-- Visual divisor using pure HTML to make page boundaries obvious -->
    <hr size="4" color="#000000">
    <center><b id="page-indicator">Loading...</b></center>
    <hr size="4" color="#000000">

    <div id="osmd-container"></div>
    
    <script>
        let currentZoom = 1.0; 
        let currentPage = 1;
        let totalPages = 1;
        let osmd = null;
        let boxWidth = 0;
        let boxHeight = 0;

        function renderMusicXmlBase64(base64Xml) {
            const container = document.getElementById("osmd-container");
            boxWidth = window.innerWidth;
            
            // Critical fix: Subtract offsetTop so the music doesn't overlap your <hr> tags
            boxHeight = window.innerHeight - container.offsetTop;
            if (boxHeight <= 0) boxHeight = 800; // Fallback if Compose hasn't laid out yet

            const xmlString = decodeURIComponent(escape(window.atob(base64Xml)));
            
            if (!osmd) {
                osmd = new opensheetmusicdisplay.OpenSheetMusicDisplay("osmd-container", {
                    autoResize: false,
                    backend: "canvas",
                    drawTitle: true,
                    drawingParameters: "compact",
                    drawPartNames: false,
                    pageBackgroundColor: "#ffffff"
                });
                
                osmd.rules.PageBottomMargin = 20.0;
                osmd.rules.PageRightMargin = 3.0;
            }
            
            osmd.setCustomPageFormat(boxWidth, boxHeight);
            
            document.getElementById("loading-overlay").style.display = "flex";
            setTimeout(function() {
                osmd.load(xmlString).then(function() {
                    osmd.zoom = currentZoom;
                    osmd.render();
                    
                    updatePageCount();
                    showPage(1);
                    document.getElementById("loading-overlay").style.display = "none";
                }).catch(function(error) {
                    console.error("OSMD Load Error: ", error);
                    document.getElementById("page-indicator").innerText = "Error Loading XML";
                    document.getElementById("loading-overlay").innerText = "Error Loading Music";
                });
            }, 50);
        }
        
        function updatePageCount() {
            const container = document.getElementById("osmd-container");
            let count = 0;
            // OSMD wraps each page in a <div>, and the cursor is an <img>. 
            // We count the divs to get the true page count.
            for (let i = 0; i < container.children.length; i++) {
                if (container.children[i].tagName.toLowerCase() === 'div') {
                    count++;
                }
            }
            totalPages = count;
        }

        function showPage(pageNumber) {
            if (totalPages === 0) return;
            
            currentPage = Math.max(1, Math.min(pageNumber, totalPages));
            
            // Slide the container up mathematically instead of hiding it
            const container = document.getElementById("osmd-container");
            const xOffset = -(currentPage - 1) * boxWidth;
            container.style.transition = "transform 0.3s ease-out";
            container.style.transform = "translateX(" + xOffset + "px)";
            document.getElementById("page-indicator").innerText = "Page " + currentPage + " of " + totalPages;
        }

        function nextPage() { if (currentPage < totalPages) showPage(currentPage + 1); }
        function prevPage() { if (currentPage > 1) showPage(currentPage - 1); }
        
        function setZoom(zoomValue) {
            if (osmd) {
                currentZoom = Math.max(0.25, Math.min(zoomValue, 2.0));
                osmd.zoom = currentZoom;
                osmd.render(); 
                updatePageCount();
                showPage(currentPage);
            }
        }

        let touchStartX = 0, touchStartY = 0, touchEndX = 0, touchEndY = 0;

        document.addEventListener('touchstart', function(event) {
            touchStartX = event.changedTouches[0].screenX;
            touchStartY = event.changedTouches[0].screenY;
        }, { passive: true });

        document.addEventListener('touchend', function(event) {
            touchEndX = event.changedTouches[0].screenX;
            touchEndY = event.changedTouches[0].screenY;
            handleSwipe();
        }, { passive: true });

        function handleSwipe() {
            const xDiff = touchStartX - touchEndX;
            const yDiff = touchStartY - touchEndY;
            
            if (Math.abs(xDiff) > Math.abs(yDiff) && Math.abs(xDiff) > 50) {
                if (xDiff > 0) nextPage();
                else prevPage();
            }
        }

        function jumpMeasure(measureNumber) {
            if (!osmd) return;
            const targetIndex = Math.max(0, measureNumber - 1);
            osmd.cursor.show();
            osmd.cursor.reset();
            
            const originalUpdate = osmd.cursor.update;
            osmd.cursor.update = function() {}; 
            
            while (osmd.cursor.iterator.CurrentMeasureIndex < targetIndex && !osmd.cursor.iterator.EndReached) {
                osmd.cursor.next();
            }
            
            osmd.cursor.update = originalUpdate;
            osmd.cursor.update();
            _syncPageWithCursor();
        }

        function increaseMeasure() {
            if (!osmd || !osmd.cursor) return;
            osmd.cursor.show();
            const targetIndex = osmd.cursor.iterator.CurrentMeasureIndex + 1;
            
            const originalUpdate = osmd.cursor.update;
            osmd.cursor.update = function() {};
            
            while (osmd.cursor.iterator.CurrentMeasureIndex < targetIndex && !osmd.cursor.iterator.EndReached) {
                osmd.cursor.next();
            }
            
            osmd.cursor.update = originalUpdate;
            osmd.cursor.update();
            _syncPageWithCursor();
        }

        function _syncPageWithCursor() {
            if (!osmd || !osmd.cursor || !osmd.cursor.cursorElement) return;
            
            const cursorX = parseFloat(osmd.cursor.cursorElement.style.left);
            if (isNaN(cursorX) || boxWidth === 0) return;
            const targetPage = Math.floor(cursorX / boxWidth) + 1;
            if (currentPage !== targetPage) {
                showPage(targetPage);
            }
        }
    </script>
</body>
</html>"""

@SuppressLint("SetJavaScriptEnabled")
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
    val xml = xmlContent.trimIndent()
    var webViewRef by remember { mutableStateOf<WebView?>(null) }
    var isPageLoaded by remember { mutableStateOf(false) }

    if (!isPageLoaded){
        Box(modifier = Modifier.fillMaxSize()) {
            CircularProgressIndicator(
                modifier = Modifier.align(Alignment.Center),
                color = Color.Black
            )
        }
    }

    LaunchedEffect(xml, isPageLoaded) {
        if (isPageLoaded && xml.isNotBlank()) {
            val base64Xml = Base64.encodeToString(xml.toByteArray(Charsets.UTF_8), Base64.NO_WRAP)
            val jsCommand = "renderMusicXmlBase64('$base64Xml');"
            webViewRef?.evaluateJavascript(jsCommand, null)
        }
    }

    LaunchedEffect(isDragging){
        if (!isDragging) {
            // Fix: Re-calculate the offsetTop so dragging doesn't break the layout math
            webViewRef?.evaluateJavascript(
                """
                const container = document.getElementById("osmd-container");
                boxWidth = window.innerWidth;
                boxHeight = window.innerHeight - container.offsetTop;
                osmd.setCustomPageFormat(boxWidth, boxHeight);
                osmd.render();
                updatePageCount();
                showPage(currentPage);
                """.trimIndent(), null
            )
        }
    }

    LaunchedEffect(zoom) {
        webViewRef?.evaluateJavascript("setZoom($zoom);", null)
    }

    AndroidView(
        modifier = modifier,
        factory = { context ->
            WebView(context).apply {
                settings.javaScriptEnabled = true
                settings.domStorageEnabled = true

                webViewClient = object : WebViewClient() {
                    override fun onPageFinished(view: WebView?, url: String?) {
                        super.onPageFinished(view, url)
                        isPageLoaded = true
                    }
                }

                loadDataWithBaseURL(
                    "https://localhost",
                    OSMD_HTML,
                    "text/html",
                    "UTF-8",
                    null
                )

                webViewRef = this
            }
        },
        onRelease = {
            webViewRef = null
            it.destroy()
        },
    )
}

