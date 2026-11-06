allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

// Puts KGP on the root classpath so the 2.4.0 build outranks the older KGP that
// plugin modules (android_file_picker) declare in their own buildscript; Gradle
// resolves a plugin applied with `apply(plugin = ...)` from the parent
// classloader first. The version must match settings.gradle.kts.
buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.0")
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}
subprojects {
    project.evaluationDependsOn(":app")
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
