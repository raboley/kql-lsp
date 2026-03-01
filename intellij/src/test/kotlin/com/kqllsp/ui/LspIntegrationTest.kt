package com.kqllsp.ui

import com.intellij.remoterobot.RemoteRobot
import com.intellij.remoterobot.stepsProcessing.StepLogger
import com.intellij.remoterobot.stepsProcessing.StepWorker
import com.intellij.remoterobot.utils.waitFor
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.MethodOrderer
import org.junit.jupiter.api.Order
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.TestInstance
import org.junit.jupiter.api.TestMethodOrder
import java.time.Duration

/**
 * UI integration tests for the KQL LSP plugin.
 *
 * These tests open all slice example files from docs/examples/ and verify
 * the LSP handles them without crashes. This catches regressions like the CRLF
 * StringIndexOutOfBoundsException bug that only appeared with real Windows files.
 *
 * Prerequisites:
 *   1. Build the LSP binary: cd ../lsp && cargo build --release
 *   2. Build the plugin: ./gradlew build
 *   3. Start the IDE sandbox: ./gradlew runIdeForUiTests
 *   4. Run these tests: ./gradlew uiTest
 *
 * IMPORTANT: The --add-opens JVM args in build.gradle.kts are required for JDK 17+.
 * Without them, GSON reflection fails with "Unable to create converter for RetrieveResponse".
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@TestMethodOrder(MethodOrderer.OrderAnnotation::class)
class LspIntegrationTest {

    private lateinit var robot: RemoteRobot

    // ====== CONFIGURE THESE ======
    private val pluginId = "com.kqllsp"
    private val lspBinaryName = "kql-lsp"
    private val home = System.getProperty("user.home").replace('\\', '/')
    private val lspLogPath = "$home/git/kql-lsp/lsp/app.log"
    private val examplesSource = "$home/git/kql-lsp/docs/examples"
    private val testProjectDir = "$home/git/kql-lsp/intellij/test-project"
    // =============================

    private val exampleFiles = listOf(
        "slice-01-document-store.kql",
        "slice-02-diagnostics.kql",
        "slice-03-semantic-tokens.kql",
        "slice-04-document-symbols.kql",
        "slice-05-completion.kql",
        "slice-06-where-expressions.kql",
        "slice-07-project-extend.kql",
        "slice-08-summarize-functions.kql",
        "slice-09-hover.kql",
        "slice-10-string-ops.kql",
        "slice-11-go-to-definition.kql",
        "slice-12-find-references.kql",
        "slice-13-join.kql",
        "slice-14-management-commands.kql",
        "slice-15-multi-statement.kql",
        "slice-16-signature-help.kql",
        "slice-17-code-action.kql",
        "slice-18-formatting.kql",
        "slice-19-folding.kql",
        "slice-20-rename.kql",
        "slice-21-schema-load.kql",
        "slice-23-column-completion.kql",
    )

    /** Character offset into app.log recorded before opening example files. */
    private var logOffsetBeforeTests: Int = 0

    /** Helper: execute JS in the IDE and return whether the result equals "true" */
    private fun callJsBool(script: String): Boolean {
        return robot.callJs<String>(script) == "true"
    }

    /** Read the entire LSP log as a string. */
    private fun readFullLog(): String {
        return runCatching {
            robot.callJs<String>("""
                importClass(java.nio.file.Files)
                importClass(java.nio.file.Paths)
                var logPath = Paths.get("$lspLogPath")
                if (Files.exists(logPath)) {
                    new java.lang.String(Files.readAllBytes(logPath))
                } else {
                    ""
                }
            """.trimIndent())
        }.getOrDefault("")
    }

    /** Read only the log content written after [logOffsetBeforeTests]. */
    private fun readLogSinceOffset(): String {
        val fullLog = readFullLog()
        return if (fullLog.length > logOffsetBeforeTests) {
            fullLog.substring(logOffsetBeforeTests)
        } else {
            ""
        }
    }

    /** Open a file in the IDE via invokeLater (non-blocking). */
    private fun openFileInIde(filePath: String) {
        robot.runJs("""
            importClass(com.intellij.openapi.project.ProjectManager)
            importClass(com.intellij.openapi.fileEditor.FileEditorManager)
            importClass(com.intellij.openapi.vfs.LocalFileSystem)
            importClass(com.intellij.openapi.application.ApplicationManager)

            var project = ProjectManager.getInstance().getOpenProjects()[0]
            var vFile = LocalFileSystem.getInstance().refreshAndFindFileByPath("$filePath")

            if (vFile != null) {
                ApplicationManager.getApplication().invokeLater(function() {
                    FileEditorManager.getInstance(project).openFile(vFile, true)
                })
            } else {
                throw new Error("Could not find file: $filePath")
            }
        """.trimIndent())
    }

    @BeforeAll
    fun setUp() {
        StepWorker.registerProcessor(StepLogger())
        val host = System.getProperty("robot.host", "http://localhost:8082")
        robot = RemoteRobot(host)

        // Wait for IDE to be ready
        println("Waiting for IDE to be ready...")
        waitFor(Duration.ofMinutes(2), Duration.ofSeconds(5)) {
            val result = runCatching {
                robot.callJs<String>(""" "hello" """)
            }
            if (result.isFailure) {
                println("  callJs failed: ${result.exceptionOrNull()?.message}")
            }
            result.isSuccess
        }
        println("IDE is responding to robot commands")
    }

    @Test
    @Order(1)
    fun `01 - IDE is running and accessible`() {
        val ideVersion = robot.callJs<String>("""
            importClass(com.intellij.openapi.application.ApplicationInfo)
            ApplicationInfo.getInstance().getFullVersion()
        """.trimIndent())
        println("IDE version: $ideVersion")
        assertTrue(ideVersion.isNotEmpty(), "IDE version should not be empty")
    }

    @Test
    @Order(2)
    fun `02 - KQL LSP plugin is installed`() {
        val result = callJsBool("""
            importClass(com.intellij.ide.plugins.PluginManagerCore)
            importClass(com.intellij.openapi.extensions.PluginId)
            var pluginId = PluginId.findId("$pluginId")
            var installed = pluginId != null && PluginManagerCore.getPlugin(pluginId) != null
            "" + installed
        """.trimIndent())
        println("KQL LSP plugin installed: $result")
        assertTrue(result, "KQL LSP plugin should be installed")
    }

    @Test
    @Order(3)
    fun `03 - lsp4ij plugin is installed`() {
        val result = callJsBool("""
            importClass(com.intellij.ide.plugins.PluginManagerCore)
            importClass(com.intellij.openapi.extensions.PluginId)
            var pluginId = PluginId.findId("com.redhat.devtools.lsp4ij")
            var installed = pluginId != null && PluginManagerCore.getPlugin(pluginId) != null
            "" + installed
        """.trimIndent())
        println("LSP4IJ plugin installed: $result")
        assertTrue(result, "LSP4IJ plugin should be installed")
    }

    @Test
    @Order(4)
    fun `04 - record log offset and wait for project`() {
        // Wait for a project to be available (IDE restores last project on startup)
        println("Waiting for a project to be available...")
        waitFor(Duration.ofMinutes(2), Duration.ofSeconds(3)) {
            runCatching {
                callJsBool("""
                    importClass(com.intellij.openapi.project.ProjectManager)
                    "" + (ProjectManager.getInstance().getOpenProjects().length > 0)
                """.trimIndent())
            }.getOrDefault(false)
        }
        println("Project is available")

        // Record current log length so subsequent tests only check new entries
        logOffsetBeforeTests = readFullLog().length
        println("Log offset recorded: $logOffsetBeforeTests chars")
    }

    @Test
    @Order(5)
    fun `05 - open all example files`() {
        // Copy example files into the test-project directory so lsp4ij can
        // see them within its project scope (lsp4ij only sends didOpen for
        // files that belong to the open project).
        println("Copying ${exampleFiles.size} example files into test-project...")
        for (file in exampleFiles) {
            robot.runJs("""
                importClass(java.nio.file.Files)
                importClass(java.nio.file.Paths)
                importClass(java.nio.file.StandardCopyOption)
                var src = Paths.get("$examplesSource/$file")
                var dst = Paths.get("$testProjectDir/$file")
                Files.copy(src, dst, StandardCopyOption.REPLACE_EXISTING)
            """.trimIndent())
        }
        // Refresh the VFS so IntelliJ sees the new files
        robot.runJs("""
            importClass(com.intellij.openapi.vfs.LocalFileSystem)
            importClass(java.io.File)
            LocalFileSystem.getInstance().refreshIoFiles(
                java.util.Collections.singletonList(new File("$testProjectDir")),
                false, true, null
            )
        """.trimIndent())
        Thread.sleep(5000) // Give lsp4ij time to notice the new files
        println("Files copied and VFS refreshed. Now opening them in the IDE...")

        for ((index, file) in exampleFiles.withIndex()) {
            val filePath = "$testProjectDir/$file"
            println("  [${index + 1}/${exampleFiles.size}] Opening $file")
            runCatching { openFileInIde(filePath) }
                .onFailure { println("    WARNING: Failed to open $file: ${it.message}") }
            Thread.sleep(1500)
        }

        // Wait for the last file to appear in the LSP log
        val lastFile = exampleFiles.last()
        println("Waiting for LSP to process last file ($lastFile)...")
        waitFor(Duration.ofMinutes(3), Duration.ofSeconds(3)) {
            val log = readLogSinceOffset()
            log.contains(lastFile)
        }
        println("All example files opened and last file processed by LSP")
    }

    @Test
    @Order(6)
    fun `06 - verify LSP processed all 20 files`() {
        val log = readLogSinceOffset()
        println("Checking LSP log for all ${exampleFiles.size} files...")

        val missingFiles = mutableListOf<String>()
        for (file in exampleFiles) {
            if (!log.contains(file)) {
                missingFiles.add(file)
            }
        }

        if (missingFiles.isEmpty()) {
            println("All ${exampleFiles.size} example files were processed by the LSP")
        } else {
            println("Missing files in log: $missingFiles")
            println("Log since offset (first 3000 chars):\n${log.take(3000)}")
        }

        assertTrue(missingFiles.isEmpty(),
            "LSP should have processed all example files. Missing: $missingFiles")
    }

    @Test
    @Order(7)
    fun `07 - verify no LSP errors in log`() {
        val log = readLogSinceOffset()
        val errorLines = log.lines().filter { it.contains(" ERROR ") }

        if (errorLines.isEmpty()) {
            println("No ERROR entries in LSP log — all files processed cleanly")
        } else {
            println("Found ${errorLines.size} ERROR entries:")
            errorLines.forEach { println("  $it") }
        }

        assertTrue(errorLines.isEmpty(),
            "LSP log should contain no ERROR entries after processing all example files. " +
            "Errors found:\n${errorLines.joinToString("\n")}")
    }

    @Test
    @Order(8)
    fun `08 - verify semantic tokens handled for all files`() {
        val log = readLogSinceOffset()
        val semanticTokenCount = log.lines().count { it.contains("semanticTokens/full") }

        println("Found $semanticTokenCount semanticTokens/full requests in log")

        // lsp4ij requests semanticTokens/full at least once per file open
        assertTrue(semanticTokenCount >= exampleFiles.size,
            "Expected at least ${exampleFiles.size} semanticTokens/full requests " +
            "(one per file), got $semanticTokenCount. " +
            "This may indicate a crash during semantic token computation (e.g., CRLF bug).")
    }

    @Test
    @Order(9)
    fun `09 - verify LSP process still running`() {
        println("Checking LSP binary is still alive after processing all files...")

        val lspRunning = runCatching {
            callJsBool("""
                importClass(java.lang.ProcessHandle)
                var found = false
                var iter = ProcessHandle.allProcesses().iterator()
                while (iter.hasNext()) {
                    var p = iter.next()
                    var cmd = p.info().command()
                    if (cmd.isPresent() && cmd.get().contains("$lspBinaryName")) {
                        found = true
                        break
                    }
                }
                "" + found
            """.trimIndent())
        }.getOrDefault(false)

        println("LSP binary running: $lspRunning")
        assertTrue(lspRunning,
            "LSP binary '$lspBinaryName' should still be running after processing all example files")
    }
}
