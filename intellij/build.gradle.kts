plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.25"
    id("org.jetbrains.intellij.platform") version "2.11.0"
}

group = providers.gradleProperty("pluginGroup").get()
version = providers.gradleProperty("pluginVersion").get()

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

repositories {
    mavenCentral()
    maven("https://packages.jetbrains.team/maven/p/ij/intellij-dependencies")
    intellijPlatform {
        defaultRepositories()
    }
}

val remoteRobotVersion = "0.11.23"

dependencies {
    intellijPlatform {
        intellijIdeaCommunity(providers.gradleProperty("platformVersion"))

        // lsp4ij provides all the LSP integration
        plugin("com.redhat.devtools.lsp4ij", "0.19.2")

        pluginVerifier()
    }

    // UI test dependencies (for runIdeForUiTests)
    testImplementation("com.intellij.remoterobot:remote-robot:$remoteRobotVersion")
    testImplementation("com.intellij.remoterobot:remote-fixtures:$remoteRobotVersion")
    testImplementation("org.junit.jupiter:junit-jupiter-api:5.10.2")
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.10.2")
    testImplementation("com.squareup.okhttp3:logging-interceptor:4.12.0")
}

intellijPlatform {
    pluginConfiguration {
        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
            untilBuild = provider { null }
        }
    }
    buildSearchableOptions = false
}

// UI test IDE configuration
intellijPlatformTesting {
    runIde {
        register("runIdeForUiTests") {
            task {
                jvmArgumentProviders += CommandLineArgumentProvider {
                    listOf(
                        "-Drobot-server.port=8082",
                        "-Dide.mac.message.dialogs.as.sheets=false",
                        "-Djb.privacy.policy.text=<!--999.999-->",
                        "-Djb.consents.confirmation.enabled=false",
                        "-Didea.trust.all.projects=true",
                        "-Dide.show.tips.on.startup.default.value=false",
                        "-Didea.initially.ask.config=false"
                    )
                }
                // Open the test-project on startup so the IDE doesn't exit immediately
                args(project.file("test-project").absolutePath)
            }
            plugins {
                robotServerPlugin()
            }
        }
    }
}

tasks.test {
    useJUnitPlatform()
    exclude("**/ui/**")
}

tasks.register<Test>("uiTest") {
    useJUnitPlatform()
    include("**/ui/**")

    // Fix JDK 17+ module access for GSON reflection (used by remote-robot's Retrofit)
    jvmArgs(
        "--add-opens", "java.base/java.lang=ALL-UNNAMED",
        "--add-opens", "java.base/java.util=ALL-UNNAMED"
    )

    systemProperty("robot.host", "http://localhost:8082")
    testLogging {
        showStandardStreams = true
        events("passed", "skipped", "failed")
    }
}
