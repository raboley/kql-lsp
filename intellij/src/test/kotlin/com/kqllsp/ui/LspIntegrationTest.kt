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
}
