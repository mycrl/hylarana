package com.github.mycrl.hylarana.app

import android.annotation.SuppressLint
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.RelativeLayout
import androidx.webkit.WebViewAssetLoader

@SuppressLint("ViewConstructor", "SetJavaScriptEnabled")
class Frontend(private val context: Context, private val bridge: Bridge) : WebView(context) {
    companion object {
        const val URI: String = "https://assets.hylarana.local/assets/index.html"
        const val JS_INTERFACE: String = "MessageTransport"
    }

    private val assetLoader = WebViewAssetLoader.Builder()
        .setDomain("assets.hylarana.local")
        .addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(context))
        .addPathHandler("/res/", WebViewAssetLoader.ResourcesPathHandler(context))
        .build()

    init {
        /**
         * Proxy request in response to a local file.
         *
         * Because the webview loads a static directory, the request is intercepted here and the
         * response is customized.
         */
        this.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(
                view: WebView?,
                req: WebResourceRequest?
            ): WebResourceResponse? {
                return req?.let { assetLoader.shouldInterceptRequest(it.url) }
            }
        }

        this.settings.javaScriptEnabled = true
        this.settings.domStorageEnabled = true

        /**
         * Hide the horizontal scrollbar and vertical scrollbar, this webview is adaptive to the
         * screen and is scrollbar free.
         */
        this.isVerticalScrollBarEnabled = false
        this.isHorizontalScrollBarEnabled = false

        /**
         * There is no height inside the webview by default, here you set the content of the webvie
         * to fill the screen. This is to prevent the webview content from collapsing and
         * overlapping.
         */
        this.layoutParams = RelativeLayout.LayoutParams(
            RelativeLayout.LayoutParams.MATCH_PARENT,
            RelativeLayout.LayoutParams.MATCH_PARENT
        )

        setWebContentsDebuggingEnabled(true)

        /**
         * This function is called back when a message needs to be sent to the webview in the call
         * bridge.
         *
         * Here the javascript statement is executed directly. It's not very efficient, but it's
         * fine for now, since there are no very performance-demanding situations with the
         * current call.
         */
        bridge.setHandler { message ->
            Handler(Looper.getMainLooper()).post {
                this.evaluateJavascript("$JS_INTERFACE.on(`$message`)") {}
            }
        }

        /**
         * Adds a javascript interface to the webview that is called when the webview needs to pass
         * a message to the application.
         */
        this.addJavascriptInterface(object {
            @JavascriptInterface
            fun send(message: String) {
                bridge.sendMessage(message)
            }
        }, JS_INTERFACE)

        this.loadUrl(URI)
    }
}
