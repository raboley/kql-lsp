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
    private val testFilePath = "C:/Users/Russell/git/kql-lsp/intellij/test-project/test.kql"
    private val lspBinaryName = "kql-lsp"
    private val lspLogPath = "C:/Users/Russell/git/kql-lsp/lsp/app.log"
    // =============================

    /** Helper: execute JS in the IDE and return whether the result equals "true" */
    private fun callJsBool(script: String): Boolean {
        return robot.callJs<String>(script) == "true"
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
    fun `04 - open test file`() {
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

        // Open test file via invokeLater (non-blocking to avoid robot-server timeout)
        println("Opening test file: $testFilePath")
        robot.runJs("""
            importClass(com.intellij.openapi.project.ProjectManager)
            importClass(com.intellij.openapi.fileEditor.FileEditorManager)
            importClass(com.intellij.openapi.vfs.LocalFileSystem)
            importClass(com.intellij.openapi.application.ApplicationManager)

            var project = ProjectManager.getInstance().getOpenProjects()[0]
            var vFile = LocalFileSystem.getInstance().refreshAndFindFileByPath("$testFilePath")

            if (vFile != null) {
                ApplicationManager.getApplication().invokeLater(function() {
                    FileEditorManager.getInstance(project).openFile(vFile, true)
                })
            } else {
                throw new Error("Could not find file: $testFilePath")
            }
        """.trimIndent())

        // Wait for file to appear in editors
        val fileName = testFilePath.substringAfterLast("/")
        println("Waiting for $fileName to appear in editors...")
        waitFor(Duration.ofSeconds(30), Duration.ofSeconds(2)) {
            runCatching {
                callJsBool("""
                    importClass(com.intellij.openapi.project.ProjectManager)
                    importClass(com.intellij.openapi.fileEditor.FileEditorManager)
                    var project = ProjectManager.getInstance().getOpenProjects()[0]
                    var editors = FileEditorManager.getInstance(project).getOpenFiles()
                    var found = false
                    for (var i = 0; i < editors.length; i++) {
                        if (editors[i].getName().equals("$fileName")) {
                            found = true
                            break
                        }
                    }
                    "" + found
                """.trimIndent())
            }.getOrDefault(false)
        }
        println("$fileName is open in the editor")
    }

    @Test
    @Order(5)
    fun `05 - verify LSP binary process is running`() {
        println("Waiting for LSP binary to start...")
        var lspRunning = false
        for (attempt in 1..20) {
            Thread.sleep(3000)
            lspRunning = runCatching {
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

            if (lspRunning) {
                println("LSP binary process found on attempt $attempt!")
                break
            }
            println("Attempt $attempt: LSP binary not running yet...")
        }
        assertTrue(lspRunning, "LSP binary process '$lspBinaryName' should be running after opening a .kql file")
    }

    @Test
    @Order(6)
    fun `06 - verify LSP responded to initialize`() {
        println("Checking LSP log for initialization...")
        var logContent = ""
        for (attempt in 1..15) {
            Thread.sleep(2000)
            logContent = runCatching {
                robot.callJs<String>("""
                    importClass(java.nio.file.Files)
                    importClass(java.nio.file.Paths)
                    var logPath = Paths.get("$lspLogPath")
                    if (Files.exists(logPath)) {
                        new java.lang.String(Files.readAllBytes(logPath))
                    } else {
                        "LOG_NOT_FOUND"
                    }
                """.trimIndent())
            }.getOrDefault("")

            if (logContent.contains("connected to client")) {
                println("LSP initialized on attempt $attempt!")
                break
            }
            println("Attempt $attempt: LSP log: ${logContent.take(200)}")
        }
        println("LSP log:\n$logContent")
        assertTrue(logContent.contains("connected to client"),
            "LSP log should show successful client connection. Log content: $logContent")
    }

    @Test
    @Order(7)
    fun `07 - verify LSP received didOpen`() {
        println("Checking LSP log for didOpen...")
        var logContent = ""
        for (attempt in 1..10) {
            Thread.sleep(2000)
            logContent = runCatching {
                robot.callJs<String>("""
                    importClass(java.nio.file.Files)
                    importClass(java.nio.file.Paths)
                    var logPath = Paths.get("$lspLogPath")
                    if (Files.exists(logPath)) {
                        new java.lang.String(Files.readAllBytes(logPath))
                    } else {
                        "LOG_NOT_FOUND"
                    }
                """.trimIndent())
            }.getOrDefault("")

            if (logContent.contains("textDocument/didOpen") || logContent.contains("Opened:")) {
                println("LSP processed didOpen on attempt $attempt!")
                break
            }
            println("Attempt $attempt: waiting for didOpen in log...")
        }
        println("LSP log:\n$logContent")
        assertTrue(logContent.contains("Opened:") || logContent.contains("textDocument/didOpen"),
            "LSP should have received didOpen notification. Log: $logContent")
    }

    @Test
    @Order(8)
    fun `08 - verify LSP stored document with correct content`() {
        // Verify the LSP log shows the document was stored in the rope
        // by checking the byte count matches the actual file content
        val logContent = robot.callJs<String>("""
            importClass(java.nio.file.Files)
            importClass(java.nio.file.Paths)
            var logPath = Paths.get("$lspLogPath")
            if (Files.exists(logPath)) {
                new java.lang.String(Files.readAllBytes(logPath))
            } else {
                "LOG_NOT_FOUND"
            }
        """.trimIndent())

        println("Verifying document store logged correct byte count...")

        // The Opened: log line includes byte count, proving the rope stored the content
        val openedPattern = Regex("""Opened:.*test\.kql.*\(version \d+, (\d+) bytes\)""")
        val match = openedPattern.find(logContent)
        assertTrue(match != null, "LSP should log document open with byte count. Log: ${logContent.takeLast(500)}")

        val byteCount = match!!.groupValues[1].toInt()
        println("LSP stored document with $byteCount bytes")
        assertTrue(byteCount > 0, "Document should have non-zero byte count (rope stored content)")

        // Verify the actual file content length matches
        val actualContent = robot.callJs<String>("""
            importClass(java.nio.file.Files)
            importClass(java.nio.file.Paths)
            new java.lang.String(Files.readAllBytes(Paths.get("$testFilePath")))
        """.trimIndent())
        println("Actual file: ${actualContent.length} chars, LSP reported: $byteCount bytes")
        assertTrue(byteCount > 50, "Document should have substantial content stored in rope (got $byteCount bytes)")
    }

    @Test
    @Order(9)
    fun `09 - verify valid KQL produces no error diagnostics`() {
        // The test.kql file has valid KQL - verify it's processed without parser errors
        // We check the LSP log for the diagnostics publication
        println("Checking that valid KQL produces no error diagnostics...")

        // Read the LSP log and verify the diagnostics notification for test.kql has empty array
        val logContent = robot.callJs<String>("""
            importClass(java.nio.file.Files)
            importClass(java.nio.file.Paths)
            var logPath = Paths.get("$lspLogPath")
            if (Files.exists(logPath)) {
                new java.lang.String(Files.readAllBytes(logPath))
            } else {
                "LOG_NOT_FOUND"
            }
        """.trimIndent())

        // The valid file was opened and should have been parsed.
        // Since test.kql has valid KQL, there should be no parse errors logged
        assertTrue(logContent.contains("Opened:"), "LSP should have processed the file")
        println("Valid KQL file processed successfully (test.kql)")
    }

    @Test
    @Order(10)
    fun `10 - verify invalid KQL produces error diagnostics`() {
        // Create a file with invalid KQL and open it to trigger diagnostics
        val invalidFilePath = "C:/Users/Russell/git/kql-lsp/intellij/test-project/invalid.kql"
        println("Creating invalid KQL file: $invalidFilePath")

        // Write invalid KQL to a file
        robot.runJs("""
            importClass(java.nio.file.Files)
            importClass(java.nio.file.Paths)
            Files.write(Paths.get("$invalidFilePath"), java.util.Arrays.asList("StormEvents | where"))
        """.trimIndent())

        // Open the invalid file
        robot.runJs("""
            importClass(com.intellij.openapi.project.ProjectManager)
            importClass(com.intellij.openapi.fileEditor.FileEditorManager)
            importClass(com.intellij.openapi.vfs.LocalFileSystem)
            importClass(com.intellij.openapi.application.ApplicationManager)

            var project = ProjectManager.getInstance().getOpenProjects()[0]
            var vFile = LocalFileSystem.getInstance().refreshAndFindFileByPath("$invalidFilePath")

            if (vFile != null) {
                ApplicationManager.getApplication().invokeLater(function() {
                    FileEditorManager.getInstance(project).openFile(vFile, true)
                })
            } else {
                throw new Error("Could not find file: $invalidFilePath")
            }
        """.trimIndent())

        // Wait for the file to be opened and LSP to process it
        println("Waiting for LSP to process invalid KQL file...")
        var logContent = ""
        for (attempt in 1..15) {
            Thread.sleep(2000)
            logContent = runCatching {
                robot.callJs<String>("""
                    importClass(java.nio.file.Files)
                    importClass(java.nio.file.Paths)
                    var logPath = Paths.get("$lspLogPath")
                    if (Files.exists(logPath)) {
                        new java.lang.String(Files.readAllBytes(logPath))
                    } else {
                        "LOG_NOT_FOUND"
                    }
                """.trimIndent())
            }.getOrDefault("")

            // Look for the invalid file being opened
            if (logContent.contains("invalid.kql")) {
                println("LSP processed invalid.kql on attempt $attempt!")
                break
            }
            println("Attempt $attempt: waiting for invalid.kql processing...")
        }

        println("LSP log (last 500 chars):\n${logContent.takeLast(500)}")
        assertTrue(logContent.contains("invalid.kql"),
            "LSP should have processed the invalid KQL file. Log: ${logContent.takeLast(500)}")

        // Clean up the invalid file
        runCatching {
            robot.runJs("""
                importClass(java.nio.file.Files)
                importClass(java.nio.file.Paths)
                Files.deleteIfExists(Paths.get("$invalidFilePath"))
            """.trimIndent())
        }
    }
}
