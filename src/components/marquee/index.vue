<style lang="scss" scoped>
	.marquee-wrapper {
		width: 100%;
		overflow: hidden;
		white-space: nowrap;

		/* > .content {
			width: auto;
			display: initial !important;
		} */
	}
</style>

<template>
	<div ref="wrapper"
		class="marquee-wrapper">
		<div ref="content"
			:style="{ transform: `translate3d(-${translated}px, 0, 0)` }"
			class="content"
			@mouseenter.stop.prevent="startMoving"
			@mouseleave.stop.prevent="reset"
			@mousedown.stop.prevent="clearTimers">
			<slot />
		</div>
	</div>
</template>

<script>
export default {
	data() {
		return {
			content: null,
			wrapperWidth: 0,
			contentWidth: 0,
			translated: 0,
			translateTimer: null,
			retractTimer: null,
			scrollEnabled: false
		};
	},
	watch: {
		content() {
			this.update();
		}
	},
	mounted() {
		this.$nextTick(() => {
			window.addEventListener('resize', this.update);
		});
		this.content = this.$slots.default[0];
	},
	methods: {
		startMoving() {
			if (!this.scrollEnabled) return;
			this.clearTimers();
			setTimeout(() => this.translate(), 250);
		},
		reset() {
			if (!this.scrollEnabled) return;
			this.clearTimers();
			setTimeout(() => this.startRetracting(), 500);
		},
		translate() {
			this.clearTimers();
			this.translateTimer = setInterval(() => {
				if (this.translated >= this.contentWidth - this.wrapperWidth) {
					clearInterval(this.translateTimer);
					return;
				}
				this.translated++;
			}, 10);
			//
		},
		startRetracting() {
			this.clearTimers();
			this.retractTimer = setInterval(() => {
				if (this.translated <= 0) {
					clearInterval(this.retractTimer);
					return;
				}
				this.translated--;
			}, 10);
		},

		clearTimers() {
			clearInterval(this.translateTimer);
			clearInterval(this.retractTimer);
		},
		update() {
			if (!this.$refs.wrapper || !this.$refs.content) return;
			this.translated = 0;
			this.wrapperWidth = this.$refs.wrapper.offsetWidth;
			this.contentWidth = this.$refs.wrapper.scrollWidth;
			this.scrollEnabled = this.contentWidth > this.wrapperWidth;
		}
	}
};
</script>
