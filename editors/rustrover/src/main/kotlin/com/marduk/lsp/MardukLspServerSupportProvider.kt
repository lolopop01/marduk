package com.marduk.lsp

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspServerSupportProvider
import com.intellij.platform.lsp.api.LspServerSupportProvider.LspServerStarter

@Suppress("UnstableApiUsage")
class MardukLspServerSupportProvider : LspServerSupportProvider {
    override fun fileOpened(
        project: Project,
        file: VirtualFile,
        serverStarter: LspServerStarter,
    ) {
        if (file.extension == "mkml") {
            serverStarter.ensureServerStarted(MardukLspServerDescriptor(project))
        }
    }
}
