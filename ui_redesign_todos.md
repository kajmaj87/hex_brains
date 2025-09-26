# UI Redesign Todos for Hex Brains Top Section

## Overview
This document outlines actionable tasks to rework the top UI section of the Hex Brains GUI based on UX analysis and design recommendations. The redesign focuses on minimalism, intuitive navigation, clear labeling, responsive design, and progressive disclosure, addressing pain points like clutter and lack of feedback.

## Todo List
1. [ ] Refactor Row 1 into Primary Toolbar and Status Bar
2. [ ] Implement Collapsible Display Settings for Row 2
3. [ ] Convert Row 3 to Window Menu Bar
4. [ ] Add Comprehensive Tooltips and Visual Feedback
5. [ ] Ensure Responsive Design and Accessibility

## Detailed Task Descriptions

### 1. Refactor Row 1 into Primary Toolbar and Status Bar
**Objectives**: Replace the cluttered first row with a clean toolbar containing play/pause button, speed controls, add snakes, and batch simulate buttons, followed by a compact status bar for metrics. Use icons for buttons and dynamic states (e.g., play morphs to pause).

**Dependencies**: None (direct code changes in gui/src/main.rs).

**Estimated Effort**: 4 hours (moderate refactoring of UI layout and logic).

**Success Criteria**: Toolbar renders with icons, buttons respond to simulation state (e.g., pause when running), status bar shows truncated metrics with tooltips, and layout adapts to window width without overflow.

### 2. Implement Collapsible Display Settings for Row 2
**Objectives**: Move color pickers to a collapsible "Display Settings" section in the second row, promoting minimalism and progressive disclosure.

**Dependencies**: Task 1 completed (to integrate with new layout).

**Estimated Effort**: 2 hours (add collapsible UI component using egui::CollapsingHeader).

**Success Criteria**: Colors are hidden by default, expandable on click, and pickers function identically to current implementation.

### 3. Convert Row 3 to Window Menu Bar
**Objectives**: Replace individual buttons with a menu bar (Simulation, View, Tools, Help) for better organization and progressive disclosure.

**Dependencies**: None (can be done independently).

**Estimated Effort**: 3 hours (implement egui menus and reorganize button logic).

**Success Criteria**: Menus dropdown correctly, checkmarks indicate open windows, and all original functionality is preserved.

### 4. Add Comprehensive Tooltips and Visual Feedback
**Objectives**: Add tooltips to all buttons, labels, and menus; implement visual states (e.g., button color changes for active sim).

**Dependencies**: Tasks 1-3 completed (to apply to new elements).

**Estimated Effort**: 2 hours (add ui.tooltip() calls and state-based styling).

**Success Criteria**: Hovering any element shows helpful text; buttons visually indicate state (e.g., play button green when paused).

### 5. Ensure Responsive Design and Accessibility
**Objectives**: Make layout wrap on narrow windows, support keyboard navigation, and high contrast.

**Dependencies**: All previous tasks.

**Estimated Effort**: 1 hour (test and adjust egui responsive features).

**Success Criteria**: UI works on 800px width (wraps elements), Tab navigation works, and passes basic accessibility checks (e.g., contrast ratios).