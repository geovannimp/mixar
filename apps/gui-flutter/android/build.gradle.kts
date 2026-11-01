allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

// Puts KGP on the root classpath so this script can configure the Kotlin
// extension of plugins that apply it imperatively (below). The version must
// match settings.gradle.kts; it also outranks the 1.8.22 that file_picker
// declares in its own buildscript.
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

// file_picker 11 applies KGP only when AGP is below 9, assuming AGP 9's built-in
// Kotlin compiles it. This project keeps `android.builtInKotlin=false` (AGP 9
// bundles Kotlin 2.2.10, below Flutter's 2.2.20 floor), so its Kotlin sources
// would never compile. Apply KGP here once the plugin has added AGP; the KGP
// version comes from the parent classpath (2.4.0, see settings.gradle.kts),
// which outranks the 1.8.22 the plugin declares in its own buildscript.
subprojects {
    if (name == "file_picker") {
        plugins.withId("com.android.library") {
            apply(plugin = "org.jetbrains.kotlin.android")
            // file_picker skips its own `kotlinOptions.jvmTarget` on AGP 9, so
            // KGP would default to the JDK's (21) while javac stays at 17.
            extensions.configure<org.jetbrains.kotlin.gradle.dsl.KotlinAndroidProjectExtension>(
                "kotlin",
            ) {
                compilerOptions.jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
            }
        }
    }
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
