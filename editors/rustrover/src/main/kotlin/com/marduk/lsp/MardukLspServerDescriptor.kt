package com.marduk.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import java.io.File

@Suppress("UnstableApiUsage")
class MardukLspServerDescriptor(project: Project)
    : ProjectWideLspServerDescriptor(project, "Marduk LSP") {

    override fun isSupportedFile(file: VirtualFile): Boolean =
        file.extension == "mkml"

    override fun createCommandLine(): GeneralCommandLine {
        // Prefer a pre-built binary (fast startup).
        findBinary()?.let { return GeneralCommandLine(it) }

        // Fall back to `cargo run` so the plugin works straight from the repo
        // without a manual build step (first launch will be slow while compiling).
        val cargo = if (isWindows) "cargo.exe" else "cargo"
        return GeneralCommandLine(
            cargo, "run",
            "--manifest-path", "${project.basePath}/Cargo.toml",
            "--package", "marduk-lsp",
            "--quiet",
        )
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    private fun findBinary(): String? {
        val base = project.basePath ?: return null
        val exe  = if (isWindows) "marduk-lsp.exe" else "marduk-lsp"
        return listOf(
            "$base/target/release/$exe",
            "$base/target/debug/$exe",
        ).firstOrNull { File(it).exists() }
    }

    private val isWindows: Boolean
        get() = System.getProperty("os.name").lowercase().contains("windows")
}
