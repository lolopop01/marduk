package com.marduk.lsp

import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

object MardukIcons {
    // Lazy + findIcon (returns null instead of throwing) avoids crashing the
    // file-type registration if the icon resource isn't resolved yet.
    val FILE: Icon? by lazy {
        IconLoader.findIcon("/icons/mkml.svg", MardukIcons::class.java)
    }
}
