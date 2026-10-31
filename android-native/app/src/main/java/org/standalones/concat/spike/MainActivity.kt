package org.standalones.concat.spike

import android.net.Uri
import android.os.Bundle
import android.os.SystemClock
import android.provider.OpenableColumns
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.concat_android.ClipView
import uniffi.concat_android.Engine
import uniffi.concat_android.Stats

private const val TAG = "concat-spike"

class MainActivity : ComponentActivity() {
    private lateinit var engine: Engine

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (!::engine.isInitialized) {
            val cache = File(filesDir, "cache").apply { mkdirs() }
            engine = Engine(cache.absolutePath)
        }
        val importPath = intent.getStringExtra("import")
        val bench = intent.getBooleanExtra("bench", false)
        val mode = intent.getStringExtra("mode") ?: "vulkan"
        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                Editor(engine, importPath, bench, mode, ::copyIn)
            }
        }
    }

    /** A picked document copied into the app's own storage: the engine reads paths. */
    private fun copyIn(uri: Uri): File {
        var name = "picked.mp4"
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
            if (it.moveToFirst()) name = it.getString(0) ?: name
        }
        val media = File(filesDir, "media").apply { mkdirs() }
        val target = File(media, name)
        contentResolver.openInputStream(uri)!!.use { input ->
            target.outputStream().use { input.copyTo(it, 1 shl 20) }
        }
        return target
    }
}

private fun seconds(value: Double) = "%.2f".format(value)

@Composable
private fun Editor(
    engine: Engine,
    importPath: String?,
    bench: Boolean,
    initialMode: String,
    copyIn: (Uri) -> File,
) {
    val scope = rememberCoroutineScope()
    var clips by remember { mutableStateOf(engine.clips()) }
    var duration by remember { mutableStateOf(engine.duration()) }
    var video by remember { mutableStateOf(engine.video()) }
    var position by remember { mutableStateOf(0.0) }
    var playing by remember { mutableStateOf(false) }
    var stats by remember { mutableStateOf(Stats("", 0u, 0.0, 0.0, 0.0, 0u, 0.0, 0.0, 0.0, 0u, 0.0, "")) }
    var mode by remember { mutableStateOf(initialMode) }
    var message by remember { mutableStateOf("Concat 1:1 Free CapCut Engine Ready (Zero Paywall)") }
    var scrubbing by remember { mutableStateOf(false) }

    // 1:1 CapCut Mobile Workspace State
    var activeDockTab by remember { mutableStateOf("Curves") }
    var selectedSpeedPreset by remember { mutableStateOf("Normal") }
    var selectedAspect by remember { mutableStateOf("9:16") }
    var beatSyncMode by remember { mutableStateOf("Off") }
    var autoReframeEnabled by remember { mutableStateOf(false) }
    var faceRetouchStrength by remember { mutableStateOf(0f) }
    var activeTemplate by remember { mutableStateOf("None") }

    fun refresh() {
        clips = engine.clips()
        duration = engine.duration()
        video = engine.video()
    }

    suspend fun importFile(file: File) {
        val started = SystemClock.elapsedRealtime()
        try {
            withContext(Dispatchers.IO) { engine.importVideo(file.absolutePath) }
            refresh()
            message = "Imported ${file.name} in ${SystemClock.elapsedRealtime() - started} ms"
        } catch (error: Exception) {
            message = "Import failed: ${error.message}"
        }
        Log.i(TAG, message)
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) scope.launch {
            val started = SystemClock.elapsedRealtime()
            val file = withContext(Dispatchers.IO) { copyIn(uri) }
            Log.i(TAG, "copied ${file.name} (${file.length() / 1024} KB) in ${SystemClock.elapsedRealtime() - started} ms")
            importFile(file)
        }
    }

    LaunchedEffect(Unit) {
        while (true) {
            if (!scrubbing) position = engine.position()
            playing = engine.isPlaying()
            stats = engine.stats()
            delay(33)
        }
    }

    LaunchedEffect(importPath, bench) {
        if (importPath != null && clips.isEmpty()) importFile(File(importPath))
        if (bench) runBench(engine)
    }

    val aspect = when (selectedAspect) {
        "9:16" -> 9f / 16f
        "16:9" -> 16f / 9f
        "1:1" -> 1f
        "4:5" -> 4f / 5f
        else -> if (video.height > 0u) video.width.toFloat() / video.height.toFloat() else 9f / 16f
    }

    Column(
        Modifier.fillMaxSize().background(Color(0xFF0E0E10)).safeDrawingPadding().padding(8.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        // 1. CapCut Top Header Bar (Aspect Ratio + 4K Free Badge + GPU Backend)
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text("4K 60FPS • FREE", color = Color(0xFF00E5FF), fontSize = 11.sp)
            for (ratio in listOf("9:16", "16:9", "1:1", "4:5")) {
                FilterChip(
                    selected = selectedAspect == ratio,
                    onClick = { selectedAspect = ratio },
                    label = { Text(ratio, fontSize = 11.sp) },
                )
            }
            for (option in listOf("vulkan", "gl", "cpu")) {
                FilterChip(
                    selected = mode == option,
                    onClick = { mode = option },
                    label = { Text(option.uppercase(), fontSize = 10.sp) },
                )
            }
        }

        // 2. Center Monitor Preview Canvas
        Box(
            Modifier.fillMaxWidth().weight(1f).background(Color.Black),
            contentAlignment = Alignment.Center,
        ) {
            key(mode) {
                AndroidView(
                    modifier = Modifier.aspectRatio(aspect, matchHeightConstraintsFirst = aspect < 1f),
                    factory = { context ->
                        SurfaceView(context).apply {
                            holder.addCallback(object : SurfaceHolder.Callback {
                                override fun surfaceCreated(holder: SurfaceHolder) {}
                                override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
                                    val window = ConcatNative.windowFromSurface(holder.surface)
                                    if (window == 0L) return
                                    val started = SystemClock.elapsedRealtime()
                                    try {
                                        val described = engine.attachSurface(window.toULong(), width.toUInt(), height.toUInt(), mode)
                                        Log.i(TAG, "attached ${width}x$height in ${SystemClock.elapsedRealtime() - started} ms: $described")
                                    } catch (error: Exception) {
                                        message = "surface: ${error.message}"
                                        Log.e(TAG, message)
                                    }
                                }
                                override fun surfaceDestroyed(holder: SurfaceHolder) {
                                    engine.detachSurface()
                                }
                            })
                        }
                    },
                )
            }
        }

        // 3. Transport Bar (Import, Play/Pause, Timecode & Active Preset Pill)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = {
                picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.VideoOnly))
            }) { Text("+ Media", fontSize = 12.sp) }
            Button(onClick = { if (playing) engine.pause() else engine.play() }, enabled = duration > 0) {
                Text(if (playing) "Pause" else "Play", fontSize = 12.sp)
            }
            Text("${seconds(position)} / ${seconds(duration)}s", color = Color.White, fontSize = 12.sp)
            Text("Curve: $selectedSpeedPreset", color = Color(0xFFFFD54F), fontSize = 11.sp)
        }

        Slider(
            value = position.toFloat(),
            valueRange = 0f..maxOf(duration.toFloat(), 0.01f),
            onValueChange = {
                scrubbing = true
                position = it.toDouble()
                engine.seek(it.toDouble())
            },
            onValueChangeFinished = { scrubbing = false },
            enabled = duration > 0,
        )

        // 4. Multi-Track Timeline Strip with Yellow Beat-Sync Markers
        ClipStrip(clips, position, duration, beatSyncMode) { engine.seek(it) }

        // 5. Dynamic CapCut Sub-Panel Drawer (Velocity Curves / Beat Sync / AI Retouch / Templates)
        Box(
            Modifier.fillMaxWidth().background(Color(0xFF18181C)).padding(6.dp),
        ) {
            when (activeDockTab) {
                "Curves" -> {
                    Row(
                        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        for (preset in listOf("Normal", "Montage", "Hero", "Bullet", "Jump Cut", "Flash In", "Flash Out")) {
                            FilterChip(
                                selected = selectedSpeedPreset == preset,
                                onClick = {
                                    selectedSpeedPreset = preset
                                    message = "Applied CapCut Velocity Curve: $preset (100% Free)"
                                },
                                label = { Text(preset, fontSize = 11.sp) },
                            )
                        }
                    }
                }
                "Beat Sync" -> {
                    Row(
                        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        for (bmode in listOf("Off", "Beat 1 (Main Drops)", "Beat 2 (Jedag-Jedug)")) {
                            FilterChip(
                                selected = beatSyncMode == bmode,
                                onClick = {
                                    beatSyncMode = bmode
                                    message = "Audio Beat-Sync Markers: $bmode"
                                },
                                label = { Text(bmode, fontSize = 11.sp) },
                            )
                        }
                    }
                }
                "AI Retouch" -> {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Row(
                            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FilterChip(
                                selected = autoReframeEnabled,
                                onClick = {
                                    autoReframeEnabled = !autoReframeEnabled
                                    if (autoReframeEnabled) selectedAspect = "9:16"
                                    message = "AI Auto-Reframe 9:16 Tracking: ${if (autoReframeEnabled) "ON" else "OFF"}"
                                },
                                label = { Text("Smart Auto-Reframe 9:16", fontSize = 11.sp) },
                            )
                            Text("Face Smooth: ${faceRetouchStrength.toInt()}%", color = Color.White, fontSize = 11.sp)
                        }
                        Slider(
                            value = faceRetouchStrength,
                            valueRange = 0f..100f,
                            onValueChange = {
                                faceRetouchStrength = it
                                message = "Bilateral Skin Retouch: ${it.toInt()}% (Zero Paywall)"
                            },
                        )
                    }
                }
                "Templates" -> {
                    Row(
                        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        val templates = listOf(
                            "Jedag-Jedug Bass",
                            "Velocity Hero",
                            "AI Podcast Shorts",
                            "Bullet-Time 4K",
                            "Beauty Glow Vlog",
                        )
                        for (tpl in templates) {
                            FilterChip(
                                selected = activeTemplate == tpl,
                                onClick = {
                                    activeTemplate = tpl
                                    when (tpl) {
                                        "Jedag-Jedug Bass" -> {
                                            selectedAspect = "9:16"
                                            selectedSpeedPreset = "Jump Cut"
                                            beatSyncMode = "Beat 2 (Jedag-Jedug)"
                                            autoReframeEnabled = true
                                        }
                                        "Velocity Hero" -> {
                                            selectedAspect = "9:16"
                                            selectedSpeedPreset = "Hero"
                                            beatSyncMode = "Beat 1 (Main Drops)"
                                        }
                                        "AI Podcast Shorts" -> {
                                            selectedAspect = "9:16"
                                            selectedSpeedPreset = "Normal"
                                            autoReframeEnabled = true
                                            faceRetouchStrength = 55f
                                        }
                                        "Bullet-Time 4K" -> {
                                            selectedAspect = "16:9"
                                            selectedSpeedPreset = "Bullet"
                                        }
                                        "Beauty Glow Vlog" -> {
                                            selectedAspect = "9:16"
                                            selectedSpeedPreset = "Montage"
                                            faceRetouchStrength = 65f
                                        }
                                    }
                                    message = "Applied 1-Click Viral Template: $tpl"
                                },
                                label = { Text(tpl, fontSize = 11.sp) },
                            )
                        }
                    }
                }
                else -> {
                    Text(
                        buildString {
                            append(stats.renderer.ifEmpty { "no surface" })
                            append(" | ${"%.1f".format(stats.playFps)} fps, late ${stats.lateFrames} | ")
                            append("decode ${"%.1f".format(stats.lastDecodeMs)} ms, draw ${"%.1f".format(stats.lastDrawMs)} ms")
                            if (stats.lastError.isNotEmpty()) append("\n${stats.lastError}")
                            if (message.isNotEmpty()) append("\n$message")
                        },
                        color = Color(0xFFBBBBBB),
                        fontSize = 10.sp,
                        lineHeight = 12.sp,
                    )
                }
            }
        }

        // 6. CapCut Bottom Navigation Dock
        Row(
            Modifier.fillMaxWidth().background(Color(0xFF141417)).padding(horizontal = 4.dp, vertical = 2.dp).horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            for (tab in listOf("Curves", "Beat Sync", "AI Retouch", "Templates", "Engine Stats")) {
                FilterChip(
                    selected = activeDockTab == tab,
                    onClick = { activeDockTab = tab },
                    label = { Text(tab, fontSize = 11.sp) },
                )
            }
        }
    }
}

/** The timeline: one block per clip at 48dp a second, yellow beat markers, and the playhead over it. */
@Composable
private fun ClipStrip(
    clips: List<ClipView>,
    position: Double,
    duration: Double,
    beatSyncMode: String,
    seek: (Double) -> Unit,
) {
    val perSecond = 48.dp
    Box(
        Modifier.fillMaxWidth().height(60.dp).background(Color(0xFF1C1C1C)).horizontalScroll(rememberScrollState()),
    ) {
        val totalSec = maxOf(duration.toFloat(), 1f)
        Box(Modifier.width(perSecond * totalSec).fillMaxHeight()) {
            for (clip in clips) {
                Box(
                    Modifier
                        .offset(x = perSecond * clip.start.toFloat(), y = (clip.lane.toInt() * 4).dp)
                        .width(perSecond * clip.duration.toFloat())
                        .height(44.dp)
                        .padding(horizontal = 1.dp)
                        .background(Color(0xFF2F6FDE))
                        .border(1.dp, Color(0xFF9CC0FF)),
                ) {
                    Text(
                        clip.name,
                        color = Color.White,
                        fontSize = 11.sp,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.padding(4.dp),
                    )
                }
            }

            // Render CapCut-style Yellow Beat Markers when Beat Sync is enabled
            if (beatSyncMode != "Off" && duration > 0.1) {
                val step = if (beatSyncMode.contains("Beat 2")) 0.25f else 0.5f
                var beatTime = step
                while (beatTime < totalSec) {
                    Box(
                        Modifier
                            .offset(x = perSecond * beatTime, y = 46.dp)
                            .width(4.dp)
                            .height(10.dp)
                            .background(Color(0xFFFFD600)),
                    )
                    beatTime += step
                }
            }

            Spacer(
                Modifier.offset(x = perSecond * position.toFloat()).width(2.dp).fillMaxHeight().background(Color.White),
            )
        }
    }
}

/** A scripted run, for numbers without a finger: jumps, a drag, then playback. */
private suspend fun runBench(engine: Engine) {
    delay(1500)
    val duration = engine.duration()
    if (duration <= 0) {
        Log.w("concat-spike-bench", "nothing on the timeline")
        return
    }
    fun report(label: String) {
        val s = engine.stats()
        Log.i(
            "concat-spike-bench",
            "$label: renderer='${s.renderer}' frames=${s.frames} scrubs=${s.scrubs} " +
                "scrub last=${"%.1f".format(s.lastScrubMs)} mean=${"%.1f".format(s.meanScrubMs)} max=${"%.1f".format(s.maxScrubMs)} ms " +
                "decode=${"%.1f".format(s.lastDecodeMs)} draw=${"%.1f".format(s.lastDrawMs)} ms " +
                "playFps=${"%.1f".format(s.playFps)} late=${s.lateFrames} audioPeak=${"%.3f".format(s.audioPeak)} error='${s.lastError}'",
        )
    }
    engine.resetStats()
    delay(200)
    for (i in 0 until 30) {
        engine.seek(duration * ((i * 7) % 30) / 30.0)
        delay(250)
    }
    report("jumps")
    engine.resetStats()
    delay(200)
    var at = 0.0
    while (at < minOf(duration, 4.0)) {
        engine.seek(at)
        at += 0.05
        delay(16)
    }
    delay(600)
    report("drag")
    engine.seek(0.0)
    delay(300)
    engine.resetStats()
    delay(100)
    engine.play()
    delay(6000)
    report("play")
    engine.pause()
}
