import React, { useCallback } from "react";
import type { ManagedDialogProps } from "./DialogProvider.tsx";
import { Dialog } from "./Dialog.tsx";

export interface ConfirmDialogProps extends ManagedDialogProps<boolean> {
	title?: string;
	message?: string;
	messageHtml?: string;
	primaryButton?: string;
	primaryButtonClassName?: string | null;
	cancelButton?: string;
}

export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
																title,
																message,
																messageHtml,
																primaryButton,
																primaryButtonClassName,
																cancelButton,
																shown,
																onClose,
															}) => {
	const handlePrimaryButtonClick = useCallback(() => {
		onClose?.(true);
	}, [onClose]);
	
	const handleClose = useCallback(() => {
		onClose?.(false);
	}, [onClose]);
	
	return (
		<Dialog
			shown={shown}
			onClose={handleClose}
			title={title}
			primaryButton={primaryButton}
			primaryButtonClassName={primaryButtonClassName}
			onPrimaryButtonClick={handlePrimaryButtonClick}
			cancelButton={cancelButton}
		>
			{message?.split("\n").map((line, i) => (
				<p key={i} className="text-justify">{line}</p>
			))}
			{messageHtml !== undefined && (
				<p className="text-justify" dangerouslySetInnerHTML={{__html: messageHtml}} />
			)}
		</Dialog>
	);
};
