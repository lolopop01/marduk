plugins {
    id("org.jetbrains.intellij.platform") version "2.11.0"
    kotlin("jvm") version "2.1.20"
}

group   = "com.marduk"
version = "0.1.0"

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        intellijIdeaUltimate("2024.3")
    }
}

// No settings pages → skip the searchable-options build (saves ~40 s and
// suppresses the spurious "modules.lsp not installed" log from the sandbox).
tasks.buildSearchableOptions {
    enabled = false
}

// JDK 25 is installed; Kotlin caps at JVM 21 — align both compilers.
tasks.withType<JavaCompile> {
    sourceCompatibility = "21"
    targetCompatibility = "21"
}
tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}
