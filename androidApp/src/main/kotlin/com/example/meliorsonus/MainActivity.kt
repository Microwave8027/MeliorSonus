package com.example.meliorsonus

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.arkivanov.decompose.DefaultComponentContext
import com.arkivanov.essenty.lifecycle.essentyLifecycle
import com.example.meliorsonus.ui.root.DefaultRootComponent

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        // Root component is created at the platform boundary, not inside any composable
        val rootComponent = DefaultRootComponent(
            componentContext = DefaultComponentContext(lifecycle = essentyLifecycle())
        )

        setContent {
            App(rootComponent = rootComponent)
        }
    }
}