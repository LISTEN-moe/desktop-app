<style lang="scss" scoped>
	.closeContainer {
		position: absolute;
		top: 10px;
		right: 10px;
		width: 35px;
		height: 35px;
		background: #262838;
		border-radius: 25px;
		z-index: 2;
		-webkit-app-region: no-drag;

		> svg {
			width: 25px;
		}
	}
</style>
<template>
	<div class="closeContainer">
		<Budicon icon="cross"
			y="5"
			:pointer="true"
			@click.native.stop.prevent="$emit('clicked')" />
	</div>
</template>

<script>
import Budicon from '@/components/icons/budicon';
export default {
	components: { Budicon }
};
</script>
