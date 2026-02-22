import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';

export interface TutorialStep {
	id: string;
	title: string;
	text: string;
	highlightSelector?: string;
	position: 'center' | 'top-right' | 'bottom-left' | 'bottom-right';
}

const TUTORIAL_STEPS: TutorialStep[] = [
	{ id: 'welcome', title: 'Welcome to GlobalTelco', text: 'You are now the CEO of a telecom startup. Build infrastructure, connect cities, and grow your company into a global empire.', position: 'center' },
	{ id: 'camera', title: 'Camera Controls', text: 'Click and drag to pan the map. Use the scroll wheel to zoom in and out. Explore the world to find good locations for your network.', position: 'center' },
	{ id: 'hud', title: 'Your Dashboard', text: 'The top bar shows your company finances. Watch your cash, profit per tick, and credit rating. Stay profitable to grow.', highlightSelector: '.hud-left', position: 'bottom-left' },
	{ id: 'build_node', title: 'Build Your First Node', text: 'Click the "+ Node" button in the toolbar, then click anywhere on land on the map. Choose a node type from the build menu to place infrastructure.', highlightSelector: '.build-buttons', position: 'bottom-left' },
	{ id: 'build_edge', title: 'Connect with an Edge', text: 'Click "+ Edge" to start building connections. Click a source node, then click a destination node to create a link between them.', highlightSelector: '.build-buttons', position: 'bottom-left' },
	{ id: 'revenue', title: 'Watch Revenue Flow', text: 'Connected infrastructure generates revenue from nearby population demand. More nodes + more connections = more revenue.', position: 'center' },
	{ id: 'panels', title: 'Management Panels', text: 'Use the panel buttons in the toolbar to manage your company. Check finances, view infrastructure, negotiate contracts, and research new tech.', highlightSelector: '.panel-buttons', position: 'bottom-left' },
	{ id: 'speed', title: 'Speed Controls', text: 'Use the speed controls or press Space to pause. Press 1-4 to change game speed. Pause to plan your strategy without time pressure.', position: 'center' },
	{ id: 'save', title: 'Save Your Progress', text: 'Press F5 to quick save and F9 to quick load. Auto-save runs periodically. Your progress is stored locally in the browser.', position: 'center' },
	{ id: 'ready', title: "You're Ready!", text: 'Build your network, outcompete rivals, survive disasters, and dominate the global telecom market. Good luck, CEO!', position: 'center' }
];

const completed = browser ? localStorage.getItem('gt_tutorial_completed') === 'true' : false;

export const tutorialActive = writable<boolean>(false);
export const tutorialStep = writable<number>(0);
export const tutorialCompleted = writable<boolean>(completed);

export const tutorialSteps = TUTORIAL_STEPS;

export function startTutorial() {
	tutorialStep.set(0);
	tutorialActive.set(true);
}

export function nextStep() {
	const current = get(tutorialStep);
	if (current < TUTORIAL_STEPS.length - 1) {
		tutorialStep.set(current + 1);
	} else {
		completeTutorial();
	}
}

export function prevStep() {
	const current = get(tutorialStep);
	if (current > 0) {
		tutorialStep.set(current - 1);
	}
}

export function skipTutorial() {
	completeTutorial();
}

function completeTutorial() {
	tutorialActive.set(false);
	tutorialCompleted.set(true);
	if (browser) {
		localStorage.setItem('gt_tutorial_completed', 'true');
	}
}

export function resetTutorial() {
	tutorialCompleted.set(false);
	tutorialStep.set(0);
	if (browser) {
		localStorage.removeItem('gt_tutorial_completed');
	}
}
