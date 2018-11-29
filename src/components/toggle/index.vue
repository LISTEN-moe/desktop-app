<style lang="scss" scoped>
	.buttonContainer {
		display: inline-block;

		button {

			&:before {
				content: "";
				width: 4px;
				height: 100%;
				position: absolute;
				left: 0px;
				top: 0px;
				background: #ff015b;
				opacity: 0;
			}

			&.isRight:before {
				left: unset;
				right: 0px;
			}
			&.active:before {
				opacity: 1
			}
		}
	}
</style>

<template>
	<div class="buttonContainer">
		<div class="indicator" />
		<button class="primary"
			:class="{ active, isRight, light }"
			@click="$emit('clicked')">
			<slot />
		</button>
	</div>
</template>

<script>
export default {
	props: {
		active: {
			'type': Boolean,
			'default': false
		},
		isRight: {
			'type': Boolean,
			'default': false
		},
		light: {
			'type': Boolean,
			'default': true
		}
	}
}
</script>
