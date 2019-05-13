<style lang="scss" scoped>
	@import "@/assets/styles/_colors.scss";

	.card {	min-height: unset !important; }

	button { width: unset !important;}

	.closeContainer {
		top: 30px !important;
		right: 25px !important;
	}

	.mfa .closeContainer {
		top: 60px !important;
	}

	.card .card-body {
		input, button { -webkit-app-region: no-drag; }
		overflow: hidden;
		min-width: 20rem;
		text-align: center;
		&.mfa input { text-align: center }
	}

	img {
		margin-bottom: 2rem;
	}
</style>

<template>
	<div class="modal">
		<div v-if="!showRegister && !showMfa"
			class="card dark shadow">
			<div class="card-body">
				<Close @clicked="closeWindow" />
				<img src="@/assets/images/logo-square-64.png">
				<input v-model="username"
					type="text"
					placeholder="Username / Email"
					@keyup.enter.stop.prevent="login">
				<input v-model="password"
					type="password"
					placeholder="Password"
					@keyup.enter.stop.prevent="login">

				<button class="noStyle"
					@click.stop.prevent="showRegister = true">Create account</button>
				<button class="primary light"
					@click.stop.prevent="login">Login</button>
			</div>
		</div>

		<div v-else-if="showRegister && !showMfa"
			class="card dark shadow">
			<div class="card-body">
				<Close @clicked="closeWindow" />
				<img src="@/assets/images/logo-square-64.png">
				<input v-model="username"
					type="text"
					placeholder="Username">
				<input v-model="email"
					type="text"
					placeholder="Email">
				<input v-model="password"
					type="password"
					placeholder="Password">
				<input v-model="repassword"
					type="password"
					placeholder="Password again"
					@keyup.enter.stop.prevent="login">

				<button class="noStyle"
					@click.stop.prevent="showRegister = false">Back to login</button>
				<button class="primary light"
					@click.stop.prevent="register">Create account</button>
			</div>
		</div>

		<div v-else-if="showMfa"
			class="card dark shadow">
			<div class="card-body mfa">
				<Close @clicked="closeWindow" />
				<img src="@/assets/images/logo-square-64.png">
				<input v-model="mfaToken"
					type="text"
					placeholder="2FA Code"
					@keyup.enter.stop.prevent="loginMfa">
				<button class="noStyle"
					@click.stop.prevent="showRegister = false">Back to login</button>
				<button class="primary light"
					@click.stop.prevent="loginMfa">Login</button>
			</div>
		</div>
	</div>
</template>

<script>
import login from '@/gql/mutations/login.gql';
import register from '@/gql/mutations/register.gql';
import loginMfa from '@/gql/mutations/loginMfa.gql';
import Close from '@/components/buttons/close';
import { remote, ipcRenderer } from 'electron';
import { onLogin } from '@/vue-apollo';

export default {
	components: { Close },
	data() {
		return {
			showRegister: false,
			showMfa: false,
			username: null,
			password: null,
			repassword: null,
			email: null,
			mfaToken: null,
			tempToken: null
		};
	},
	computed: {
		showLogin() {
			return this.$store.state.showLogin;
		}
	},
	methods: {
		async register() {
			if (!this.username || !this.password || !this.repassword || !this.email) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: 'Please fill the 4 fields before attempting to register', error: true, duration: 5000 }); */
				return;
			}
			if (this.password !== this.repassword) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: 'Make sure both passwords match', error: true, duration: 5000 }); */
				return;
			}
			try {
				await this.$apollo.mutate({
					mutation: register,
					variables: {
						username: this.username,
						password: this.password,
						email: this.email
					}
				});
				this.clearInputs();
				this.showRegister = false;
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', 'Successfully created an account, now please log in'); */
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}
		},
		async login() {
			if (!this.username || !this.password) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: 'Can\'t log in without both fields, you special snowflake ❄', error: true, duration: 5000 }); */
				return;
			}
			try {
				const res = await this.$apollo.mutate({
					mutation: login,
					variables: {
						username: this.username,
						password: this.password
					}
				});
				if (res.data.login.mfa) {
					this.tempToken = res.data.login.token;
					this.showMfa = true;
					return;
				}
				this.saveLoginData(res.data.login);
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}
			return remote.getCurrentWindow().close();
		},
		async loginMfa() {
			if (!this.mfaToken) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: 'Can\'t log in without mfa token, you special snowflake ❄', error: true, duration: 5000 }); */
				return;
			}
			try {
				await onLogin(this.$apollo, this.tempToken);
				const res = await this.$apollo.mutate({
					mutation: loginMfa,
					variables: { token: this.mfaToken }
				});
				this.saveLoginData(res.data.loginMFA);
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}

			return remote.getCurrentWindow().close();
		},
		async saveLoginData({ token, user }) {
			this.$store.dispatch('login', { token, user });
			ipcRenderer.send('login', { token, user });
			// TODO: Proper feedback
			/* this.$store.dispatch('alert', `Welcome back, ${response.user.displayName}`); */
			await onLogin(this.$apollo, token);
		},
		clearInputs() {
			this.username = null;
			this.password = null;
			this.repassword = null;
			this.email = null;
			this.mfaToken = null;
			this.tempToken = null;
			this.showMfa = false;
		},
		closeWindow() {
			return remote.getCurrentWindow().close();
		}
	}
};
</script>
