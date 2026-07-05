package com.stirlingmobile.ui

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

data class PipelineEntry(val path: String, val label: String)
data class PipelineUiState(val current: PipelineEntry? = null, val canUndo: Boolean = false)

/**
 * Shared across tool screens (instantiated once in MainActivity's App()) so a tool's
 * output can become the next tool's input. Undo is a plain path stack: no file
 * deletion, since each tool already writes a fresh UUID-named output file.
 */
class PipelineState : ViewModel() {
    private val undoStack = ArrayDeque<PipelineEntry>()
    private val _state = MutableStateFlow(PipelineUiState())
    val state: StateFlow<PipelineUiState> = _state

    fun push(path: String, label: String) {
        _state.value.current?.let { undoStack.addLast(it) }
        _state.value = PipelineUiState(PipelineEntry(path, label), undoStack.isNotEmpty())
    }

    fun undo() {
        val prev = undoStack.removeLastOrNull() ?: return
        _state.value = PipelineUiState(prev, undoStack.isNotEmpty())
    }

    fun clear() {
        undoStack.clear()
        _state.value = PipelineUiState()
    }
}
