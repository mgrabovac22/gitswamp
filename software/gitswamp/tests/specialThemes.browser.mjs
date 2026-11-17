import assert from "node:assert/strict";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const { chromium } = createRequire(import.meta.url)(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = fileURLToPath(new URL("..", import.meta.url));
const entry = `
import { createApp, defineAsyncComponent, h } from 'vue';
import ThemeAmbience from '/src/view/shell/ThemeAmbience.vue';
import DesktopPetSettings from '/src/features/shell/DesktopPetSettings.vue';
import { petEnabled } from '/src/features/shell/petPreferences.ts';
import { ANIMATED_SPECIAL_THEMES, APP_THEME_OPTIONS, applyAppPalettePreference, applyThemeModePreference, storeAppPalettePreference, getStoredAppPalettePreference } from '/src/shared/themePreferences.ts';
import '/src/styles/index.css';
window.scenes = ANIMATED_SPECIAL_THEMES;
window.pickTheme = (id) => {
  const theme = APP_THEME_OPTIONS.find(option => option.id === id);
  if (!theme) throw new Error('Unlisted theme: ' + id);
  applyThemeModePreference(theme.mode);
  applyAppPalettePreference(id);
  storeAppPalettePreference(id);
  return getStoredAppPalettePreference();
};
window.pickTheme('swamp');
const DesktopPet = defineAsyncComponent(() => import('/src/features/shell/DesktopPet.vue'));
createApp({ render: () => h('main', {class: 'smoke-app'}, [
  h('aside', 'GitSwamp'),
  h('section', [h('header', 'Repository history'), h('button', {class:'test-action', onClick:() => window.pageClicks = (window.pageClicks || 0) + 1}, 'Continue'), h('p', 'Update integration branch'), h('p', 'Save local work'), h(DesktopPetSettings)]),
  h(ThemeAmbience),
  petEnabled.value ? h(DesktopPet) : null,
]) }).mount('#app');`;

const bundled = await build({
  root, configFile: false, publicDir: false, logLevel: "error",
  plugins: [{ name: "special-theme-test", resolveId: id => id === "virtual:scenes" ? "\0scenes" : undefined,
    load: id => id === "\0scenes" ? entry : undefined }, vue(), tailwindcss()],
  resolve: { alias: { "@": path.join(root, "src") } },
  build: { write: false, minify: false, cssCodeSplit: false, rollupOptions: { input: "virtual:scenes" } },
});
const files = new Map(bundled.output.map(asset => ["/" + asset.fileName, asset]));
const script = "/" + bundled.output.find(asset => asset.type === "chunk" && asset.isEntry).fileName;
const css = "/" + bundled.output.find(asset => asset.fileName.endsWith(".css")).fileName;
const server = createServer((req, res) => {
  if (req.url === "/") {
    res.setHeader("Content-Type", "text/html");
    return res.end(`<!doctype html><html><head><link rel="stylesheet" href="${css}"><style>
    body { margin: 0; font: 15px system-ui; background: var(--background); color: var(--foreground); }
    .smoke-app { min-height: 100vh; display: grid; grid-template-columns: minmax(90px, 22%) 1fr; }
    aside, section { padding: 16px; }
    aside, header, p { background: var(--card); border: 1px solid var(--border); padding: 8px; }
    .test-action { background: var(--primary); color: var(--primary-foreground); margin-block: 12px; padding: 6px 12px; }
    </style></head><body><div id="app"></div><script type="module" src="${script}"></script></body></html>`);
  }
  const asset = files.get(req.url);
  if (!asset) return res.writeHead(404).end();
  res.setHeader("Content-Type", asset.type === "chunk" ? "application/javascript" : asset.fileName.endsWith(".png") ? "image/png" : "text/css");
  res.end(asset.type === "chunk" ? asset.code : asset.source);
});

let browser;
try {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  browser = await chromium.launch(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {});
  const page = await browser.newPage({ viewport: { width: 1120, height: 720 } });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const scenes = await page.evaluate(() => window.scenes);
  const styles = new Set(), animations = new Set();
  assert.equal(scenes.length, 10);
  assert.equal(await page.locator(".theme-scene").count(), 0);
  for (const scene of scenes) {
    assert.equal(await page.evaluate(id => window.pickTheme(id), scene.id), scene.id);
    const layer = page.locator(`[data-scene="${scene.id}"]`);
    await layer.waitFor();
    assert.equal(await page.locator(".theme-scene").count(), 1);
    assert.equal(await layer.locator("span").count(), scene.particles);
    const data = await layer.evaluate(element => {
      const luminance = value => value.match(/\d+/g).slice(0, 3).map(Number).reduce((sum, channel, i) => {
        const s = channel / 255;
        return sum + (s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][i];
      }, 0);
      const contrast = (a, b) => (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      const body = getComputedStyle(document.body);
      const button = getComputedStyle(document.querySelector('button'));
      return [body.backgroundColor, getComputedStyle(element.firstElementChild).animationName,
        contrast(luminance(body.color), luminance(body.backgroundColor)),
        contrast(luminance(button.color), luminance(button.backgroundColor))];
    });
    assert.notEqual(data[1], "none");
    assert.ok(data[2] >= 4.5 && data[3] >= 4.5, `${scene.label} needs readable text and primary buttons`);
    styles.add(data[0]);
    animations.add(data[1]);
    const first = layer.locator("span").first();
    const initial = await first.evaluate(element => element.getAnimations()[0].currentTime);
    await page.waitForFunction(({ id, initial }) => document.querySelector(`[data-scene="${id}"] span`).getAnimations()[0]?.currentTime > initial, { id: scene.id, initial });
  }
  assert.equal(styles.size, 10);
  assert.equal(animations.size, 10);
  await page.getByRole("button", { name: "Continue" }).click();
  assert.equal(await page.evaluate(() => window.pageClicks), 1);
  await page.setViewportSize({ width: 390, height: 740 });
  await page.evaluate(() => window.pickTheme("autumn-trail"));
  await page.locator('[data-scene="autumn-trail"]').waitFor();
  await page.screenshot({ path: path.join(root, "src-tauri/target/theme-autumn-compact.png") });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.locator(".theme-scene").waitFor({ state: "hidden" });
  assert.equal(await page.locator(".theme-scene span").first().evaluate(element => element.getAnimations().length), 0);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.setViewportSize({ width: 1120, height: 720 });
  await page.evaluate(() => window.pickTheme("lantern-festival"));
  await page.locator('[data-scene="lantern-festival"]').waitFor();
  await page.screenshot({ path: path.join(root, "src-tauri/target/theme-lantern-desktop.png") });
  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await page.locator(".theme-scene--paused").waitFor();
  assert.equal(await page.locator(".theme-scene span").first().evaluate(element => getComputedStyle(element).animationPlayState), "paused");
  await page.evaluate(() => window.pickTheme("snowy"));
  await page.locator(".theme-ambience__glyph--snow").first().waitFor();
  assert.equal(await page.locator(".theme-scene").count(), 0);
  for (const id of ["snowy", "starry-night", "pixel-arcade", "anime-sakura"]) {
    await page.evaluate(theme => window.pickTheme(theme), id);
    const crossing = page.locator(`[data-crossing="${id}"]`);
    await crossing.waitFor();
    assert.notEqual(await crossing.evaluate(element => getComputedStyle(element).animationName), "none");
    await page.emulateMedia({ reducedMotion: "reduce" });
    await crossing.waitFor({ state: "hidden" });
    await page.emulateMedia({ reducedMotion: "no-preference" });
  }
  await page.evaluate(() => window.pickTheme("swamp"));
  await page.locator(".theme-ambience").waitFor({ state: "detached" });

  await page.evaluate(() => { delete document.hidden; });
  await page.clock.install();
  const pet = page.locator(".desktop-pet");
  const petSwitch = page.getByRole("switch", { name: "Desktop pet" });
  const editPet = page.getByRole("button", { name: "Edit pet" });
  assert.equal(await pet.count(), 0);
  assert.ok(await editPet.isDisabled());
  await petSwitch.click();
  await pet.waitFor();
  const walker = pet.locator(".desktop-pet__walker");
  const floorTricks = new Set();
  const checkFloor = async (expected) => {
    assert.equal(await walker.getAttribute("data-tricks"), expected);
    expected.split(" ").forEach(trick => floorTricks.add(trick));
    assert.equal(await pet.locator("[data-pet-wall]").count(), 0);
  };
  const nextPetVisit = async () => {
    await page.clock.fastForward(20_000);
    await pet.waitFor({ state: "detached" });
    await page.clock.fastForward(100_000);
    await pet.waitFor();
  };
  const inspectFloor = async () => {
    const floor = await pet.evaluate(element => {
      const walker = element.querySelector('.desktop-pet__walker');
      const body = element.querySelector('.desktop-pet__tricks');
      const sprite = element.querySelector('svg.pet-sprite');
      const foot = element.querySelector('[data-limb="leg-near"] .pet-limb__skin');
      const animations = element.getAnimations({ subtree: true });
      const delays = animations.filter(animation => animation.effect.target === body).map(animation => animation.effect.getTiming().delay);
      sprite.pauseAnimations();
      const route = time => {
        for (const animation of animations) {
          animation.pause();
          animation.currentTime = time;
        }
        sprite.setCurrentTime(time / 1000);
        const { left, right, bottom } = walker.getBoundingClientRect();
        return { left, right, bottom, foot: foot.getBoundingClientRect().right,
          lift: new DOMMatrix(getComputedStyle(body).transform).m42 };
      };
      const start = route(0), step = route(100), end = route(20_000);
      const checkpoints = [3650, 9100, 14550].map(time => ({
        arrive: route(time), trick: route(time + 850), depart: route(time + 1800),
        peakLift: Math.min(...[450, 650, 850, 1100].map(offset => route(time + offset).lift)),
      }));
      route(9950);
      return { start, step, end, checkpoints, width: innerWidth, height: innerHeight, delays };
    });
    assert.ok(floor.start.left <= 12 && floor.end.right >= floor.width - 12 && floor.end.right <= floor.width, `The floor route must cross the whole window: ${JSON.stringify(floor)}`);
    assert.ok(floor.start.bottom > floor.height - 10 && floor.end.bottom === floor.start.bottom && floor.step.left > floor.start.left, `The pet should keep walking along the bottom: ${JSON.stringify(floor)}`);
    assert.ok(Math.abs(floor.step.foot - floor.start.foot) < 4, `The floor stride should stay planted: ${JSON.stringify(floor)}`);
    assert.deepEqual(floor.delays, [3650, 9100, 14550]);
    assert.ok(floor.checkpoints.every(({ arrive, trick, depart, peakLift }) => Math.abs(arrive.left - depart.left) < 1 && arrive.bottom === trick.bottom && peakLift < -1), `The route should pause for all three visible tricks: ${JSON.stringify(floor)}`);
  };

  await checkFloor("wave cartwheel victory");
  await inspectFloor();
  await page.screenshot({ path: path.join(root, "src-tauri/target/desktop-pet-floor.png") });
  await page.clock.fastForward(20_000);
  await pet.waitFor({ state: "detached" });
  await page.clock.fastForward(90_000);
  assert.equal(await pet.count(), 0);
  await page.setViewportSize({ width: 365, height: 670 });
  await page.clock.fastForward(10_000);
  await pet.waitFor();
  await checkFloor("star backflip balance");
  await inspectFloor();
  await page.setViewportSize({ width: 1120, height: 720 });
  await nextPetVisit();
  assert.equal(await walker.getAttribute("data-tricks"), "somersault");
  assert.equal(await pet.locator("[data-pet-wall]").count(), 2);
  assert.match(await pet.locator(".desktop-pet__walker").evaluate(element => getComputedStyle(element).animationDuration), /^20s$/);
  assert.equal(await pet.locator("svg image").count(), 1);
  assert.equal(await pet.locator("[data-limb]").count(), 4);
  assert.equal(await pet.locator(".pet-limb__texture[fill^='url(']").count(), 4);
  const pose = await pet.evaluate(element => {
    const animations = element.getAnimations({ subtree: true });
    const walker = element.querySelector('.desktop-pet__walker');
    const sprite = element.querySelector('svg.pet-sprite');
    const leg = element.querySelector('[data-limb="leg-near"] .pet-limb__skin');
    const legShape = element.querySelector('[data-limb="leg-near"] .pet-limb__geometry');
    const arm = element.querySelector('[data-limb="arm-near"] .pet-limb__skin');
    sprite.pauseAnimations();
    const poseAt = time => {
      for (const animation of animations) {
        animation.pause();
        animation.currentTime = time;
      }
      sprite.setCurrentTime(time / 1000);
    };
    const route = time => {
      poseAt(time);
      const box = walker.getBoundingClientRect();
      return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
    };
    poseAt(735);
    const before = legShape.getTotalLength();
    const grounded = leg.getBBox().y + leg.getBBox().height;
    const plantedX = leg.getBoundingClientRect().right;
    const restingArm = arm.getBBox().y;
    poseAt(900);
    const after = legShape.getTotalLength();
    const stance = leg.getBBox().y + leg.getBBox().height;
    const stanceX = leg.getBoundingClientRect().right;
    poseAt(528);
    const swing = leg.getBBox().y + leg.getBBox().height;
    const ground = walker.querySelector('.desktop-pet__tricks');
    const ropeBottom = route(3200);
    const ropeTop = route(6870);
    const crossing = route(8200);
    const rockCatch = route(9550);
    const rockRestA = route(10400);
    const rockRestB = route(11100);
    const rockTop = route(12100);
    const centre = route(16604);
    const jump = new DOMMatrix(getComputedStyle(ground).transform).m42;
    const landing = route(19310);
    const facing = walker.querySelector('.desktop-pet__facing');
    const touchdown = new DOMMatrix(getComputedStyle(facing).transform);
    poseAt(3500);
    const grip = arm.getBBox().y;
    const attached = [4800, 5720, 8610, 9550, 10180, 16604, 17800, 19310].every(time => {
      poseAt(time);
      return [...element.querySelectorAll('.pet-limb__geometry')].every(path => {
        const a = path.getPointAtLength(0), b = path.getPointAtLength(path.getTotalLength());
        const center = [(a.x + b.x) / 2, (a.y + b.y) / 2];
        const hip = path.closest('[data-limb]').dataset.limb.startsWith('leg');
        return center[0] >= 95 && center[0] <= 147 && Math.abs(center[1] - (hip ? 132 : 80)) < 3;
      });
    });
    poseAt(9550);
    return { before, after, grounded, stance, swing, plantedX, stanceX, restingArm, jump, grip, touchdown: touchdown.m22, attached,
      ropeBottom, ropeTop, crossing, rockCatch, rockRestA, rockRestB, rockTop, centre, landing, width: innerWidth, height: innerHeight };
  });
  assert.ok(Math.abs(pose.before - pose.after) > 1 && Math.abs(pose.grounded - pose.stance) < 1 && pose.stance - pose.swing > 7, `The path must bend and the foot plant before lifting: ${JSON.stringify(pose)}`);
  assert.ok(Math.abs(pose.plantedX - pose.stanceX) < 2, `The standing foot should not slide across the window: ${JSON.stringify(pose)}`);
  assert.ok(pose.ropeBottom.x > pose.width * .88 && pose.ropeBottom.y - pose.ropeTop.y > 75 && pose.restingArm - pose.grip > 12, `Swampy should reach for the right-hand rope and climb it: ${JSON.stringify(pose)}`);
  assert.ok(pose.crossing.x > pose.width * .35 && pose.crossing.x < pose.width * .8 && pose.crossing.y < pose.rockCatch.y - 20, `The leap should go across the window in an arc: ${JSON.stringify(pose)}`);
  assert.ok(pose.rockCatch.x < pose.width * .13 && pose.rockCatch.y < pose.ropeTop.y - 22 && Math.abs(pose.rockRestA.y - pose.rockRestB.y) < 1 && pose.rockTop.y < pose.rockRestA.y - 10, `The second edge should be caught higher and have an asymmetric climb: ${JSON.stringify(pose)}`);
  assert.ok(Math.abs(pose.centre.x - pose.width / 2) < 2 && Math.abs(pose.centre.y - pose.height / 2) < 12 && pose.jump < -70 && pose.landing.y > pose.height * .88 && pose.touchdown < .92, `The grand trick should peak at the centre with a compressed landing: ${JSON.stringify(pose)}`);
  assert.ok(pose.attached, "Both shoulders and hips should stay within their torso contact areas across the whole routine");
  await page.screenshot({ path: path.join(root, "src-tauri/target/desktop-pet-rock-catch.png") });
  await pet.evaluate(element => {
    for (const animation of element.getAnimations({ subtree: true })) animation.currentTime = 16604;
    element.querySelector('svg.pet-sprite').setCurrentTime(16.604);
  });
  await page.screenshot({ path: path.join(root, "src-tauri/target/desktop-pet-finale.png") });
  await editPet.click();
  const editor = page.getByRole("dialog", { name: "Edit pet" });
  await editor.waitFor();
  await editor.getByLabel("Name", { exact: true }).fill("Captain Swampy");
  await editor.getByLabel("Hat", { exact: true }).selectOption("crown");
  await editor.getByLabel("Accessory", { exact: true }).selectOption("bandana");
  await editor.getByRole("radio", { name: "River" }).check();
  await page.screenshot({ path: path.join(root, "src-tauri/target/desktop-pet-editor.png") });
  await page.setViewportSize({ width: 365, height: 670 });
  assert.ok(await editor.evaluate(element => element.scrollWidth <= element.clientWidth));
  const trickPicker = editor.getByLabel("Trick", { exact: true });
  const previewMotion = editor.getByRole("button", { name: "Preview motion" });
  const tricks = await trickPicker.locator("option").evaluateAll(options => options.map(option => option.value));
  assert.equal(tricks.length, 10);
  const routines = new Set();
  for (const trick of tricks) {
    await trickPicker.selectOption(trick);
    await previewMotion.click();
    const playing = editor.locator(`.pet-editor__sprite--trick[data-trick="${trick}"]`);
    await playing.waitFor();
    const program = await playing.evaluate(element => {
      const motion = element.getAnimations()[0];
      const svg = element.querySelector('svg');
      motion.pause();
      svg.pauseAnimations();
      const frame = motion.effect.getKeyframes().reduce((nearest, frame) =>
        Math.abs(frame.offset - .49) < Math.abs(nearest.offset - .49) ? frame : nearest);
      motion.currentTime = frame.offset * 2500;
      svg.setCurrentTime(frame.offset * 2.5);
      return motion.effect.getKeyframes().map(key => key.transform).join(';');
    });
    assert.ok(program.includes("rotate("));
    assert.ok(await playing.locator('[data-limb="arm-near"] animate[dur="2.5s"]').count());
    assert.ok(await playing.locator('.pet-tail').evaluate(tail => tail.getAnimations().some(animation => animation.effect.getTiming().duration === 2500)));
    routines.add(program);
    if (trick === "star") await page.screenshot({ path: path.join(root, "src-tauri/target/desktop-pet-star-jump.png") });
  }
  assert.equal(routines.size, 10);
  await editor.getByRole("button", { name: "Save pet" }).click();
  await editor.waitFor({ state: "detached" });
  assert.deepEqual(await page.evaluate(() => JSON.parse(localStorage.getItem("gitswamp-desktop-pet-appearance"))),
    { name: "Captain Swampy", hat: "crown", accessory: "bandana", color: "river", tricks: true });
  await nextPetVisit();
  await checkFloor("split twist somersault");
  await nextPetVisit();
  await checkFloor("balance kip wave");
  assert.deepEqual([...floorTricks].sort(), [...tricks].sort());
  await nextPetVisit();
  assert.equal(await walker.getAttribute("data-tricks"), "cartwheel");
  assert.equal(await pet.locator("[data-pet-wall]").count(), 2);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await pet.waitFor({ state: "detached" });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await pet.waitFor();
  await editPet.click();
  await editor.getByLabel("Name", { exact: true }).fill("Unsaved name");
  await page.keyboard.press("Escape");
  await editor.waitFor({ state: "detached" });
  assert.equal(await page.getByText("Captain Swampy", { exact: true }).count(), 1);
  await editPet.click();
  await editor.getByRole("checkbox", { name: "Acrobatic tricks" }).uncheck();
  assert.ok(await trickPicker.isDisabled());
  await previewMotion.click();
  await editor.locator(".pet-sprite--walking").waitFor();
  assert.equal(await editor.locator(".pet-editor__sprite--trick").count(), 0);
  await editor.getByRole("button", { name: "Save pet" }).click();
  await page.clock.fastForward(20_000);
  await pet.waitFor({ state: "detached" });
  await page.clock.fastForward(100_000);
  await pet.waitFor();
  assert.equal(await pet.locator(".desktop-pet__walker").getAttribute("data-tricks"), "");
  assert.equal(await pet.locator("[data-pet-wall]").count(), 0);
  await petSwitch.click();
  await pet.waitFor({ state: "detached" });
  await page.clock.fastForward(240_000);
  assert.equal(await pet.count(), 0);
  assert.deepEqual(errors, []);
  console.log("PASS: special themes, screen crossings, motion settings, and desktop pet editing, schedule and cleanup.");
} finally {
  const closing = Promise.all([browser?.close(), new Promise(resolve => server.close(resolve))]);
  server.closeAllConnections();
  await closing;
}
