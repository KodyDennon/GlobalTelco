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
	{ id: 'welcome', title: '', text: '', position: 'center' },
	{ id: 'camera', title: '', text: '', position: 'center' },
	{ id: 'hud', title: '', text: '', highlightSelector: '.hud-left', position: 'bottom-left' },
	{ id: 'build_node', title: '', text: '', highlightSelector: '.build-buttons', position: 'bottom-left' },
	{ id: 'build_edge', title: '', text: '', highlightSelector: '.build-buttons', position: 'bottom-left' },
	{ id: 'revenue', title: '', text: '', position: 'center' },
	{ id: 'panels', title: '', text: '', highlightSelector: '.panel-buttons', position: 'bottom-left' },
	{ id: 'speed', title: '', text: '', position: 'center' },
	{ id: 'save', title: '', text: '', position: 'center' },
	{ id: 'ready', title: '', text: '', position: 'center' }
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
