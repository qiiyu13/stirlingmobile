package com.stirlingmobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.stirlingmobile.ui.CompressScreen
import com.stirlingmobile.ui.MergeScreen
import com.stirlingmobile.ui.PagesToolMode
import com.stirlingmobile.ui.PagesToolScreen
import com.stirlingmobile.ui.PasswordToolMode
import com.stirlingmobile.ui.PasswordToolScreen
import com.stirlingmobile.ui.RotateScreen
import com.stirlingmobile.ui.SplitScreen

private enum class Tool { HOME, MERGE, SPLIT, ROTATE, REMOVE, EXTRACT, COMPRESS, ADD_PASSWORD, REMOVE_PASSWORD }

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface {
                    App()
                }
            }
        }
    }
}

@Composable
private fun App() {
    var tool by remember { mutableStateOf(Tool.HOME) }
    when (tool) {
        Tool.HOME -> HomeScreen(onSelect = { tool = it })
        Tool.MERGE -> MergeScreen()
        Tool.SPLIT -> SplitScreen()
        Tool.ROTATE -> RotateScreen()
        Tool.REMOVE -> PagesToolScreen(PagesToolMode.REMOVE)
        Tool.EXTRACT -> PagesToolScreen(PagesToolMode.EXTRACT)
        Tool.COMPRESS -> CompressScreen()
        Tool.ADD_PASSWORD -> PasswordToolScreen(PasswordToolMode.ADD)
        Tool.REMOVE_PASSWORD -> PasswordToolScreen(PasswordToolMode.REMOVE)
    }
}

@Composable
private fun HomeScreen(onSelect: (Tool) -> Unit) {
    Column(
        modifier = Modifier.padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Stirling Mobile")
        Button(onClick = { onSelect(Tool.MERGE) }) { Text("Merge") }
        Button(onClick = { onSelect(Tool.SPLIT) }) { Text("Split") }
        Button(onClick = { onSelect(Tool.ROTATE) }) { Text("Rotate") }
        Button(onClick = { onSelect(Tool.REMOVE) }) { Text("Remove Pages") }
        Button(onClick = { onSelect(Tool.EXTRACT) }) { Text("Extract Pages") }
        Button(onClick = { onSelect(Tool.COMPRESS) }) { Text("Compress") }
        Button(onClick = { onSelect(Tool.ADD_PASSWORD) }) { Text("Add Password") }
        Button(onClick = { onSelect(Tool.REMOVE_PASSWORD) }) { Text("Remove Password") }
    }
}
