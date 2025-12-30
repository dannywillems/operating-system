// Kanban column resizing functionality
(function() {
    'use strict';

    const MIN_WIDTH = 200;
    const MAX_WIDTH = 600;
    const STORAGE_KEY_PREFIX = 'kanban-column-widths-';

    // Get board ID from the page
    function getBoardId() {
        const chatContainer = document.getElementById('chat-container');
        return chatContainer ? chatContainer.dataset.boardId : null;
    }

    // Get storage key for current board
    function getStorageKey() {
        const boardId = getBoardId();
        return boardId ? STORAGE_KEY_PREFIX + boardId : null;
    }

    // Load saved column widths from localStorage
    function loadColumnWidths() {
        const key = getStorageKey();
        if (!key) return {};

        try {
            const saved = localStorage.getItem(key);
            return saved ? JSON.parse(saved) : {};
        } catch (e) {
            console.error('Failed to load column widths:', e);
            return {};
        }
    }

    // Save column widths to localStorage
    function saveColumnWidths(widths) {
        const key = getStorageKey();
        if (!key) return;

        try {
            localStorage.setItem(key, JSON.stringify(widths));
        } catch (e) {
            console.error('Failed to save column widths:', e);
        }
    }

    // Get column identifier (using column name or index)
    function getColumnId(column, index) {
        const header = column.querySelector('.kanban-column-header span');
        if (header) {
            return header.textContent.trim();
        }
        return 'column-' + index;
    }

    // Apply saved widths to columns
    function applyColumnWidths() {
        const columns = document.querySelectorAll('.kanban-column');
        const widths = loadColumnWidths();

        columns.forEach((column, index) => {
            const columnId = getColumnId(column, index);
            if (widths[columnId]) {
                column.style.width = widths[columnId] + 'px';
            }
        });
    }

    // Initialize resize handles for all columns
    function initResizeHandles() {
        const columns = document.querySelectorAll('.kanban-column');

        columns.forEach((column, index) => {
            // Skip if already has a resize handle
            if (column.querySelector('.kanban-column-resize')) return;

            const handle = document.createElement('div');
            handle.className = 'kanban-column-resize';
            handle.title = 'Drag to resize column';
            column.appendChild(handle);

            let startX = 0;
            let startWidth = 0;
            let columnId = getColumnId(column, index);

            function onMouseDown(e) {
                e.preventDefault();
                startX = e.clientX;
                startWidth = column.offsetWidth;
                columnId = getColumnId(column, index);

                handle.classList.add('kanban-column-resize--active');
                column.classList.add('kanban-column--resizing');
                document.body.style.cursor = 'col-resize';
                document.body.style.userSelect = 'none';

                document.addEventListener('mousemove', onMouseMove);
                document.addEventListener('mouseup', onMouseUp);
            }

            function onMouseMove(e) {
                const deltaX = e.clientX - startX;
                let newWidth = startWidth + deltaX;

                // Apply constraints
                newWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth));

                column.style.width = newWidth + 'px';
            }

            function onMouseUp() {
                handle.classList.remove('kanban-column-resize--active');
                column.classList.remove('kanban-column--resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';

                document.removeEventListener('mousemove', onMouseMove);
                document.removeEventListener('mouseup', onMouseUp);

                // Save the new width
                const widths = loadColumnWidths();
                widths[columnId] = column.offsetWidth;
                saveColumnWidths(widths);
            }

            handle.addEventListener('mousedown', onMouseDown);

            // Touch support for mobile
            handle.addEventListener('touchstart', function(e) {
                if (e.touches.length === 1) {
                    const touch = e.touches[0];
                    onMouseDown({ preventDefault: () => {}, clientX: touch.clientX });
                }
            }, { passive: false });
        });
    }

    // Double-click to reset column width
    function initDoubleClickReset() {
        const columns = document.querySelectorAll('.kanban-column');

        columns.forEach((column, index) => {
            const handle = column.querySelector('.kanban-column-resize');
            if (!handle) return;

            handle.addEventListener('dblclick', function() {
                const columnId = getColumnId(column, index);
                column.style.width = '300px'; // Default width

                // Update saved widths
                const widths = loadColumnWidths();
                delete widths[columnId];
                saveColumnWidths(widths);
            });
        });
    }

    // Initialize when DOM is ready
    function init() {
        const kanbanBoard = document.querySelector('.kanban-board');
        if (!kanbanBoard) return;

        applyColumnWidths();
        initResizeHandles();
        initDoubleClickReset();
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
