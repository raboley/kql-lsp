package com.kqllsp;

import com.intellij.execution.configurations.GeneralCommandLine;
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider;

public class KqlLspServer extends OSProcessStreamConnectionProvider {

    public KqlLspServer() {
        // Launch the KQL LSP binary built from the lsp/ directory.
        // The binary communicates over stdio (stdin/stdout).
        String home = System.getProperty("user.home").replace('\\', '/');
        String os = System.getProperty("os.name", "").toLowerCase();
        String ext = os.contains("win") ? ".exe" : "";
        String binaryPath = home + "/git/kql-lsp/lsp/target/release/kql-lsp" + ext;
        String workDir = home + "/git/kql-lsp/lsp";

        GeneralCommandLine commandLine = new GeneralCommandLine(binaryPath);
        commandLine.setWorkDirectory(workDir);

        super.setCommandLine(commandLine);
    }
}
