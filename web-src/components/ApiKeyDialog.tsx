import React, { useCallback, useRef } from "react";
import type { ManagedDialogProps } from "./DialogProvider.tsx";
import { Dialog } from "./Dialog.tsx";
import { useForm } from "react-hook-form";

interface ApiKeyFormData {
	apiKey: string;
}

export interface ApiKeyDialogProps extends ManagedDialogProps<string> {
}

export const ApiKeyDialog: React.FC<ApiKeyDialogProps> = ({
	shown,
	onClose,
}) => {
	const {register, handleSubmit} = useForm<ApiKeyFormData>();
	const formRef = useRef<HTMLFormElement>(null);
	
	const handleFormSubmit = useCallback((evt: React.FormEvent) => {
		handleSubmit(data => {
			onClose?.(data.apiKey);
		})(evt);
	}, []);
	
	return (
		<Dialog
			title="API key"
			primaryButton="Save"
			cancelButton="Close"
			formRef={formRef}
			shown={shown}
			onClose={onClose}
		>
			<form ref={formRef} onSubmit={handleFormSubmit}>
				<input
					type="password"
					{...register("apiKey")}
					className="input w-full"
					required
				/>
			</form>
		</Dialog>
	);
};
